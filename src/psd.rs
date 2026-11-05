//! Adobe PSD (Version 1) binary encoder, PackBits RLE scanline compressor,
//! atomic disk writer, and high-level CMYK/RGB PSD export pipelines.

use std::fs::File;
use std::io::{BufWriter, Error, ErrorKind, Result, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Maximum width and height (in pixels) supported by the Adobe PSD Version 1 specification.
pub const PSD_MAX_DIMENSION: u32 = 30_000;

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Applies image adjustments in sRGB space followed by forward CMM translation
/// using the selected intent and Black Point Compensation setting, and writes a flat CMYK PSD file.
pub fn export_cmyk_psd_file(
    input_path: &str,
    icc_path: &str,
    intent: crate::conversion::RenderingIntent,
    black_point_compensation: bool,
    adjustments: crate::adjustment::AdjustmentParams,
    output_path: &str,
) -> std::result::Result<(), String> {
    let (width, height, raw_rgba) = crate::conversion::load_normalized_srgb_image(input_path)
        .map_err(|e| format!("元画像 '{}' の読み込みに失敗しました: {}", input_path, e))?;

    let icc = crate::conversion::load_cmyk_icc(icc_path).map_err(|e| {
        format!(
            "CMYKカラープロファイル '{}' の読み込みに失敗しました: {}",
            icc_path, e
        )
    })?;

    // Apply pre-CMYK sRGB image adjustments (fast no-op if all parameters are 0)
    let rgba_pixels = crate::adjustment::apply_adjustments(
        &raw_rgba,
        width as usize,
        height as usize,
        adjustments,
    );

    let cmyk_pixels = crate::conversion::convert_srgb_to_cmyk(
        &rgba_pixels,
        width as usize,
        height as usize,
        &icc,
        intent,
        black_point_compensation,
    )
    .map_err(|e| format!("RGBからCMYKへの色変換に失敗しました: {}", e))?;

    write_cmyk_psd(width, height, &cmyk_pixels, &icc.bytes, output_path)
        .map_err(|e| format!("CMYK PSDファイルの書き込みに失敗しました: {}", e))
}

/// Normalizes the input image (embedded ICC -> sRGB and white background composite)
/// and packages the RGB pixels into a flat RGB PSD file.
pub fn export_rgb_psd_file(input_path: &str, output_path: &str) -> std::result::Result<(), String> {
    let (width, height, rgba_pixels) = crate::conversion::load_normalized_srgb_image(input_path)
        .map_err(|e| format!("元画像 '{}' の読み込みに失敗しました: {}", input_path, e))?;

    write_rgb_psd(width, height, &rgba_pixels, output_path)
        .map_err(|e| format!("RGB PSDファイルの書き込みに失敗しました: {}", e))
}

/// Validates PSD image dimensions against the PSD v1 limit (1..=30,000) and verifies
/// that the pixel slice length matches `width * height * 4` without overflow.
pub fn validate_psd_buffer(width: u32, height: u32, pixels: &[u8]) -> Result<usize> {
    if width == 0 || height == 0 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "画像サイズは 1x1 ピクセル以上である必要があります。",
        ));
    }
    if width > PSD_MAX_DIMENSION || height > PSD_MAX_DIMENSION {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!(
                "画像サイズ ({}x{}) が PSD 仕様の上限 ({}x{}) を超えています。",
                width, height, PSD_MAX_DIMENSION, PSD_MAX_DIMENSION
            ),
        ));
    }

    let total_pixels = (width as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidInput,
                "画素数の計算がオーバーフローしました。",
            )
        })?;
    let expected_bytes = total_pixels.checked_mul(4).ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidInput,
            "バッファサイズの計算がオーバーフローしました。",
        )
    })?;

    if pixels.len() != expected_bytes {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!(
                "ピクセルバッファ長 ({}) が期待値 ({}) と一致しません。",
                pixels.len(),
                expected_bytes
            ),
        ));
    }

    Ok(total_pixels)
}

/// Builds a unique temporary file path in the same directory as `output_path`.
fn make_temp_path(output_path: &Path) -> PathBuf {
    let parent = output_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = output_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("output.psd");
    let seq = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    parent.join(format!(".{}.easypsd-{}-{}.tmp", stem, pid, seq))
}

/// Writes to a sibling temporary file first, flushes and syncs to disk, and only then
/// atomically replaces `output_path` so an existing file is never corrupted on write failure.
fn write_file_atomically<F>(output_path: &str, write_fn: F) -> Result<()>
where
    F: FnOnce(&mut BufWriter<File>) -> Result<()>,
{
    let target = Path::new(output_path);
    let temp_path = make_temp_path(target);

    let write_result = (|| -> Result<()> {
        let file = File::create(&temp_path)?;
        let mut writer = BufWriter::new(file);
        write_fn(&mut writer)?;
        writer.flush()?;
        writer.get_ref().sync_all()?;
        Ok(())
    })();

    if let Err(e) = write_result {
        let _ = std::fs::remove_file(&temp_path);
        return Err(e);
    }

    if let Err(e) = std::fs::rename(&temp_path, target) {
        let _ = std::fs::remove_file(&temp_path);
        return Err(e);
    }

    Ok(())
}

/// Encodes a single scanline using the Apple PackBits (RLE) algorithm as specified by Adobe PSD (`Compression = 1`).
fn encode_packbits_row(row: &[u8], out: &mut Vec<u8>) {
    let len = row.len();
    let mut i = 0;

    while i < len {
        // Count consecutive identical bytes starting at `i` (up to 128)
        let first = row[i];
        let max_run = (len - i).min(128);
        let mut run_len = 1;
        while run_len < max_run && row[i + run_len] == first {
            run_len += 1;
        }

        // Emit a repeat packet if run_len >= 3, or if a 2-byte repeat is at the end
        // of the row or immediately followed by another repeat.
        let use_repeat = run_len >= 3
            || (run_len == 2 && (i + 2 == len || (i + 3 < len && row[i + 2] == row[i + 3])));

        if use_repeat {
            // Header byte: -(run_len - 1) in two's complement = (257 - run_len) as u8
            out.push((257 - run_len) as u8);
            out.push(first);
            i += run_len;
        } else {
            // Accumulate a literal run of up to 128 bytes until a 3+ byte repeat is encountered
            let lit_start = i;
            let mut lit_len = 0;
            while i < len && lit_len < 128 {
                if i + 2 < len && row[i] == row[i + 1] && row[i] == row[i + 2] {
                    break;
                }
                i += 1;
                lit_len += 1;
            }
            out.push((lit_len - 1) as u8);
            out.extend_from_slice(&row[lit_start..lit_start + lit_len]);
        }
    }
}

/// Writes planar channel scanlines into the PSD Image Data section using PackBits RLE (`Compression = 1`),
/// or falls back to raw uncompressed (`Compression = 0`) if any compressed scanline exceeds `u16::MAX` bytes.
fn write_psd_image_data<W: Write>(
    writer: &mut W,
    width: u32,
    height: u32,
    channels: &[Vec<u8>],
) -> Result<()> {
    let w = width as usize;
    let h = height as usize;
    let total_rows = channels.len() * h;

    let mut row_lengths: Vec<u16> = Vec::with_capacity(total_rows);
    let mut compressed_data: Vec<u8> = Vec::new();
    let mut row_buf: Vec<u8> = Vec::with_capacity(w + (w / 128) + 2);
    let mut can_use_rle = w > 0 && h > 0;

    if can_use_rle {
        'outer: for channel in channels {
            for row in channel.chunks_exact(w) {
                row_buf.clear();
                encode_packbits_row(row, &mut row_buf);
                if let Ok(byte_count) = u16::try_from(row_buf.len()) {
                    row_lengths.push(byte_count);
                    compressed_data.extend_from_slice(&row_buf);
                } else {
                    can_use_rle = false;
                    break 'outer;
                }
            }
        }
    }

    if can_use_rle {
        // Compression = 1 (RLE / PackBits)
        writer.write_all(&1u16.to_be_bytes())?;
        // Scanline byte count table: 2-byte big-endian length for every row of every channel
        for count in row_lengths {
            writer.write_all(&count.to_be_bytes())?;
        }
        // Compressed scanline payloads
        writer.write_all(&compressed_data)?;
    } else {
        // Fallback: Compression = 0 (Raw uncompressed bytes)
        writer.write_all(&0u16.to_be_bytes())?;
        for channel in channels {
            writer.write_all(channel)?;
        }
    }

    Ok(())
}

/// Writes a flattened (single-layer) CMYK PSD file with an embedded target ICC color profile.
///
/// Employs PackBits RLE compressed channel data (`Compression = 1`) written in Planar Order
/// (all Cyan scanlines, followed by all Magenta, Yellow, and Black scanlines) as required
/// by the Adobe PSD specification.
pub fn write_cmyk_psd(
    width: u32,
    height: u32,
    cmyk_pixels: &[u8],
    icc_profile: &[u8],
    output_path: &str,
) -> Result<()> {
    let total_pixels = validate_psd_buffer(width, height, cmyk_pixels)?;

    let mut channels = vec![vec![0u8; total_pixels]; 4];
    for i in 0..total_pixels {
        channels[0][i] = 255 - cmyk_pixels[i * 4];
        channels[1][i] = 255 - cmyk_pixels[i * 4 + 1];
        channels[2][i] = 255 - cmyk_pixels[i * 4 + 2];
        channels[3][i] = 255 - cmyk_pixels[i * 4 + 3];
    }

    write_file_atomically(output_path, |writer| {
        // =====================================================================
        // 1. File Header (26 bytes fixed-size)
        // =====================================================================
        writer.write_all(b"8BPS")?; // Signature: Always "8BPS"
        writer.write_all(&1u16.to_be_bytes())?; // Version: Always 1
        writer.write_all(&[0; 6])?; // Reserved: 6 bytes filled with zeros
        writer.write_all(&4u16.to_be_bytes())?; // Number of channels: 4 (Cyan, Magenta, Yellow, Black)
        writer.write_all(&height.to_be_bytes())?; // Image height - big-endian
        writer.write_all(&width.to_be_bytes())?; // Image width - big-endian
        writer.write_all(&8u16.to_be_bytes())?; // Channel depth: 8 bits per channel
        writer.write_all(&4u16.to_be_bytes())?; // Color mode: 4 represents CMYK mode

        // =====================================================================
        // 2. Color Mode Data Section
        // =====================================================================
        writer.write_all(&0u32.to_be_bytes())?; // Length = 0

        // =====================================================================
        // 3. Image Resources Section (ICC Profile Resource 0x040F)
        // =====================================================================
        let profile_len = u32::try_from(icc_profile.len()).map_err(|_| {
            Error::new(
                ErrorKind::InvalidInput,
                "ICCプロファイルサイズが大きすぎます。",
            )
        })?;
        let pad = if profile_len % 2 != 0 { 1 } else { 0 };
        let block_len = 4 + 2 + 2 + 4 + profile_len + pad;

        writer.write_all(&block_len.to_be_bytes())?;
        writer.write_all(b"8BIM")?;
        writer.write_all(&0x040Fu16.to_be_bytes())?;
        writer.write_all(&[0, 0])?;
        writer.write_all(&profile_len.to_be_bytes())?;
        writer.write_all(icc_profile)?;
        if pad > 0 {
            writer.write_all(&[0])?;
        }

        // =====================================================================
        // 4. Layer and Mask Information Section
        // =====================================================================
        writer.write_all(&0u32.to_be_bytes())?; // Section length = 0

        // =====================================================================
        // 5. Image Data Section (PackBits RLE Compressed Planar Channels)
        // =====================================================================
        write_psd_image_data(writer, width, height, &channels)
    })
}

/// Writes a flattened, simple RGB PSD file without any profile conversions.
pub fn write_rgb_psd(width: u32, height: u32, rgba_pixels: &[u8], output_path: &str) -> Result<()> {
    let total_pixels = validate_psd_buffer(width, height, rgba_pixels)?;

    let mut channels = vec![vec![0u8; total_pixels]; 3];
    for i in 0..total_pixels {
        channels[0][i] = rgba_pixels[i * 4];
        channels[1][i] = rgba_pixels[i * 4 + 1];
        channels[2][i] = rgba_pixels[i * 4 + 2];
    }

    write_file_atomically(output_path, |writer| {
        // =====================================================================
        // 1. File Header (26 bytes fixed-size)
        // =====================================================================
        writer.write_all(b"8BPS")?;
        writer.write_all(&1u16.to_be_bytes())?;
        writer.write_all(&[0; 6])?;
        writer.write_all(&3u16.to_be_bytes())?;
        writer.write_all(&height.to_be_bytes())?;
        writer.write_all(&width.to_be_bytes())?;
        writer.write_all(&8u16.to_be_bytes())?;
        writer.write_all(&3u16.to_be_bytes())?;

        // =====================================================================
        // 2. Color Mode Data Section
        // =====================================================================
        writer.write_all(&0u32.to_be_bytes())?;

        // =====================================================================
        // 3. Image Resources Section
        // =====================================================================
        writer.write_all(&0u32.to_be_bytes())?;

        // =====================================================================
        // 4. Layer and Mask Information Section
        // =====================================================================
        writer.write_all(&0u32.to_be_bytes())?;

        // =====================================================================
        // 5. Image Data Section (PackBits RLE Compressed Planar Channels)
        // =====================================================================
        write_psd_image_data(writer, width, height, &channels)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode_packbits(encoded: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < encoded.len() {
            let header = encoded[i] as i8;
            i += 1;
            if header >= 0 {
                let count = (header as usize) + 1;
                out.extend_from_slice(&encoded[i..i + count]);
                i += count;
            } else if header != -128 {
                let count = (1 - (header as isize)) as usize;
                let val = encoded[i];
                i += 1;
                out.extend(std::iter::repeat_n(val, count));
            }
        }
        out
    }

    #[test]
    fn packbits_roundtrips_various_scanline_patterns() {
        let mut long_mixed = Vec::new();
        long_mixed.extend(std::iter::repeat_n(255u8, 300));
        long_mixed.extend(0u8..=200u8);
        long_mixed.extend([10, 10, 20, 30, 30, 30, 40, 50, 50]);

        let cases: &[&[u8]] = &[
            &[],
            &[42],
            &[7, 7],
            &[7, 7, 9],
            &[1, 2, 3, 4, 5, 6, 7, 8],
            &[99; 128],
            &[99; 129],
            &long_mixed,
        ];

        for &row in cases {
            let mut encoded = Vec::new();
            encode_packbits_row(row, &mut encoded);
            let decoded = decode_packbits(&encoded);
            assert_eq!(decoded, row);
        }
    }

    #[test]
    fn packbits_significantly_compresses_uniform_and_illustration_rows() {
        let uniform_row = vec![255u8; 4000];
        let mut encoded = Vec::new();
        encode_packbits_row(&uniform_row, &mut encoded);
        assert_eq!(decode_packbits(&encoded), uniform_row);
        assert_eq!(encoded.len(), 64);
    }

    #[test]
    fn write_psd_image_data_writes_valid_rle_table_and_payload() {
        let width = 8u32;
        let height = 2u32;
        let channels = vec![
            vec![
                255, 255, 255, 255, 10, 20, 30, 40, // row 0
                0, 0, 0, 0, 0, 0, 0, 0, // row 1
            ],
            vec![
                1, 2, 3, 4, 5, 6, 7, 8, // row 0
                9, 9, 9, 9, 9, 9, 9, 9, // row 1
            ],
        ];

        let mut buf = Vec::new();
        write_psd_image_data(&mut buf, width, height, &channels).unwrap();

        assert_eq!(&buf[0..2], &1u16.to_be_bytes());

        let total_rows = 4;
        let mut offset = 2 + total_rows * 2;
        for (ch_idx, ch) in channels.iter().enumerate() {
            for (row_idx, expected_row) in ch.chunks_exact(width as usize).enumerate() {
                let table_idx = 2 + (ch_idx * 2 + row_idx) * 2;
                let byte_count = u16::from_be_bytes([buf[table_idx], buf[table_idx + 1]]) as usize;
                let decoded = decode_packbits(&buf[offset..offset + byte_count]);
                assert_eq!(decoded, expected_row);
                offset += byte_count;
            }
        }
        assert_eq!(offset, buf.len());
    }

    #[test]
    fn validate_psd_buffer_rejects_invalid_dimensions_and_mismatched_lengths() {
        assert!(validate_psd_buffer(0, 10, &[]).is_err());
        assert!(validate_psd_buffer(10, 0, &[]).is_err());
        assert!(validate_psd_buffer(PSD_MAX_DIMENSION + 1, 1, &[0; 120004]).is_err());
        assert!(validate_psd_buffer(1, PSD_MAX_DIMENSION + 1, &[0; 120004]).is_err());
        assert!(validate_psd_buffer(2, 2, &[0; 15]).is_err());
        assert_eq!(validate_psd_buffer(2, 2, &[0; 16]).unwrap(), 4);
    }

    #[test]
    fn atomic_write_preserves_existing_file_on_failure() {
        let temp_target =
            std::env::temp_dir().join(format!("easypsd_atomic_test_{}.psd", std::process::id()));
        let target_str = temp_target.to_string_lossy().to_string();

        let original_bytes = b"existing valid psd content";
        std::fs::write(&temp_target, original_bytes).unwrap();

        let res = write_rgb_psd(4, 4, &[0u8; 8], &target_str);
        assert!(res.is_err());
        assert_eq!(std::fs::read(&temp_target).unwrap(), original_bytes);

        let res2 = write_file_atomically(&target_str, |w| {
            w.write_all(b"partial")?;
            Err(Error::other("simulated disk full"))
        });
        assert!(res2.is_err());
        assert_eq!(std::fs::read(&temp_target).unwrap(), original_bytes);

        let _ = std::fs::remove_file(&temp_target);
    }
}
