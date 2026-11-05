use std::fs::File;
use std::io::{BufWriter, Write, Result};

/// Writes a flattened (single-layer) CMYK PSD file with an embedded target ICC color profile.
/// 
/// Employs raw uncompressed channel data written in Planar Order (all Cyan pixels, 
/// followed by all Magenta, Yellow, and Black pixels) as required by the Adobe PSD specification.
/// 
/// * `width`: Width of the image in pixels.
/// * `height`: Height of the image in pixels.
/// * `cmyk_pixels`: Interleaved CMYK pixel bytes (C, M, Y, K repeated).
/// * `icc_profile`: Raw binary bytes of the target ICC color profile.
/// * `output_path`: Absolute file path where the resulting PSD file is written.
pub fn write_cmyk_psd(
    width: u32,
    height: u32,
    cmyk_pixels: &[u8],
    icc_profile: &[u8],
    output_path: &str,
) -> Result<()> {
    let file = File::create(output_path)?;
    let mut writer = BufWriter::new(file);

    // =========================================================================
    // 1. File Header (26 bytes fixed-size)
    // =========================================================================
    writer.write_all(b"8BPS")?;                 // Signature: Always "8BPS"
    writer.write_all(&1u16.to_be_bytes())?;     // Version: Always 1
    writer.write_all(&[0; 6])?;                 // Reserved: 6 bytes filled with zeros
    writer.write_all(&4u16.to_be_bytes())?;     // Number of channels: 4 (Cyan, Magenta, Yellow, Black)
    writer.write_all(&height.to_be_bytes())?;   // Image height - big-endian
    writer.write_all(&width.to_be_bytes())?;    // Image width - big-endian
    writer.write_all(&8u16.to_be_bytes())?;     // Channel depth: 8 bits per channel
    writer.write_all(&4u16.to_be_bytes())?;     // Color mode: 4 represents CMYK mode

    // =========================================================================
    // 2. Color Mode Data Section
    // =========================================================================
    // Used for indexed color tables or duotones. Zero-length for standard CMYK.
    writer.write_all(&0u32.to_be_bytes())?;     // Length = 0

    // =========================================================================
    // 3. Image Resources Section
    // =========================================================================
    // Target ICC color profiles must be embedded inside an Image Resource block 
    // identified by the standard Resource ID 0x040F (1039).
    let profile_len = icc_profile.len() as u32;
    // The entire resource data block must be padded to an even number of bytes
    let pad = if profile_len % 2 != 0 { 1 } else { 0 };
    // Block size: "8BIM" (4 bytes) + ID (2 bytes) + Pascal String Name padding (2 bytes) + Size field (4 bytes) + Body + Pad
    let block_len = 4 + 2 + 2 + 4 + profile_len + pad;

    writer.write_all(&block_len.to_be_bytes())?; // Write the total length of the Image Resources Section

    // Write the specific ICC Profile resource block
    writer.write_all(b"8BIM")?;                     // Resource signature: Always "8BIM"
    writer.write_all(&0x040Fu16.to_be_bytes())?;   // Resource ID: 0x040F (1039) for ICC Profile
    writer.write_all(&[0, 0])?;                     // Resource name: Empty Pascal String (padded to even = 2 bytes of zeros)
    writer.write_all(&profile_len.to_be_bytes())?; // Length of the raw ICC profile data
    writer.write_all(icc_profile)?;                 // ICC profile binary payload
    if pad > 0 {
        writer.write_all(&[0])?;                    // Align to even byte boundary
    }

    // =========================================================================
    // 4. Layer and Mask Information Section
    // =========================================================================
    // Since we are writing a flat background image with no layered structures, 
    // we specify the section size to be zero.
    writer.write_all(&0u32.to_be_bytes())?;     // Section length = 0

    // =========================================================================
    // 5. Image Data Section
    // =========================================================================
    writer.write_all(&0u16.to_be_bytes())?;     // Compression: 0 represents raw, uncompressed bytes

    // Write channel bytes in Planar Order (Cyan, then Magenta, then Yellow, then Black).
    // 
    // Adobe CMYK PSD expects "Ink Density" semantics where 0 is 100% ink (darkest) and 255 is 0% ink (paper white).
    // lcms2 outputs standard ratio semantics where 0 is 0% ink and 255 is 100% ink.
    // To prevent print-inverted negatives, we apply `255 - value` inversion before writing channels.
    let total_pixels = (width * height) as usize;
    let mut channel_buf = vec![0u8; total_pixels];

    // Channel 0: Cyan
    for i in 0..total_pixels {
        channel_buf[i] = 255 - cmyk_pixels[i * 4];
    }
    writer.write_all(&channel_buf)?;

    // Channel 1: Magenta
    for i in 0..total_pixels {
        channel_buf[i] = 255 - cmyk_pixels[i * 4 + 1];
    }
    writer.write_all(&channel_buf)?;

    // Channel 2: Yellow
    for i in 0..total_pixels {
        channel_buf[i] = 255 - cmyk_pixels[i * 4 + 2];
    }
    writer.write_all(&channel_buf)?;

    // Channel 3: Black (Key)
    for i in 0..total_pixels {
        channel_buf[i] = 255 - cmyk_pixels[i * 4 + 3];
    }
    writer.write_all(&channel_buf)?;

    writer.flush()?;
    Ok(())
}

/// Writes a flattened, simple RGB PSD file without any profile conversions.
/// 
/// Mainly utilized when profile transformations are disabled, or when exporting 
/// a direct flat PSD representation of the original RGB source file.
/// 
/// * `width`: Width of the image in pixels.
/// * `height`: Height of the image in pixels.
/// * `rgba_pixels`: Interleaved RGBA pixel bytes (R, G, B, A repeated).
///                  Note: The Alpha channel (A) is discarded according to flat PSD specs; 
///                  only Red, Green, and Blue channels are packed and written.
/// * `output_path`: Absolute file path where the resulting PSD file is written.
pub fn write_rgb_psd(
    width: u32,
    height: u32,
    rgba_pixels: &[u8],
    output_path: &str,
) -> Result<()> {
    let file = File::create(output_path)?;
    let mut writer = BufWriter::new(file);

    // =========================================================================
    // 1. File Header (26 bytes fixed-size)
    // =========================================================================
    writer.write_all(b"8BPS")?;                 // Signature: Always "8BPS"
    writer.write_all(&1u16.to_be_bytes())?;     // Version: Always 1
    writer.write_all(&[0; 6])?;                 // Reserved
    writer.write_all(&3u16.to_be_bytes())?;     // Number of channels: 3 (Red, Green, Blue)
    writer.write_all(&height.to_be_bytes())?;   // Image height - big-endian
    writer.write_all(&width.to_be_bytes())?;    // Image width - big-endian
    writer.write_all(&8u16.to_be_bytes())?;     // Channel depth: 8 bits
    writer.write_all(&3u16.to_be_bytes())?;     // Color mode: 3 represents RGB mode

    // =========================================================================
    // 2. Color Mode Data Section
    // =========================================================================
    writer.write_all(&0u32.to_be_bytes())?;     // Length = 0

    // =========================================================================
    // 3. Image Resources Section
    // =========================================================================
    // No target ICC profiles embedded; section length set to zero.
    writer.write_all(&0u32.to_be_bytes())?;

    // =========================================================================
    // 4. Layer and Mask Information Section
    // =========================================================================
    writer.write_all(&0u32.to_be_bytes())?;     // Section length = 0 (flat background canvas)

    // =========================================================================
    // 5. Image Data Section
    // =========================================================================
    writer.write_all(&0u16.to_be_bytes())?;     // Compression: 0 represents raw, uncompressed bytes

    // Write RGB channel bytes in Planar Order (Red, then Green, then Blue).
    // Note that standard RGB PSD files do not require any value inversions (unlike CMYK).
    // 0 represents darkness and 255 represents maximum brightness.
    let total_pixels = (width * height) as usize;
    let mut channel_buf = vec![0u8; total_pixels];

    // Channel 0: Red
    for i in 0..total_pixels {
        channel_buf[i] = rgba_pixels[i * 4];
    }
    writer.write_all(&channel_buf)?;

    // Channel 1: Green
    for i in 0..total_pixels {
        channel_buf[i] = rgba_pixels[i * 4 + 1];
    }
    writer.write_all(&channel_buf)?;

    // Channel 2: Blue
    for i in 0..total_pixels {
        channel_buf[i] = rgba_pixels[i * 4 + 2];
    }
    writer.write_all(&channel_buf)?;

    writer.flush()?;
    Ok(())
}
