use std::fs;
use std::path::Path;
use std::process::Command;

/// Applies image adjustments in sRGB space followed by forward CMM translation
/// using the selected intent and Black Point Compensation setting, and writes a flat CMYK PSD file.
pub fn export_cmyk_psd_file(
    input_path: &str,
    icc_path: &str,
    intent: crate::conversion::RenderingIntent,
    black_point_compensation: bool,
    adjustments: crate::adjustment::AdjustmentParams,
    output_path: &str,
) -> Result<(), String> {
    let img = image::open(input_path)
        .map_err(|e| format!("元画像 '{}' の読み込みに失敗しました: {}", input_path, e))?;
    let width = img.width();
    let height = img.height();
    let raw_rgba = img.to_rgba8().into_raw();

    // Apply pre-CMYK sRGB image adjustments (fast no-op if all parameters are 0)
    let rgba_pixels = crate::adjustment::apply_adjustments(
        &raw_rgba,
        width as usize,
        height as usize,
        adjustments,
    );

    let icc_data = fs::read(icc_path).map_err(|e| {
        format!(
            "CMYKカラープロファイル '{}' の読み込みに失敗しました: {}",
            icc_path, e
        )
    })?;

    let cmyk_profile = lcms2::Profile::new_icc(&icc_data)
        .map_err(|e| format!("CMYKカラープロファイルの解析に失敗しました: {}", e))?;

    if cmyk_profile.color_space() != lcms2::ColorSpaceSignature::CmykData {
        return Err("選択されたプロファイルはCMYK色空間ではありません。".to_string());
    }
    let srgb_profile = lcms2::Profile::new_srgb();

    let lcms_intent = intent.to_lcms2();
    let forward_flags = crate::conversion::cmm_transform_flags(black_point_compensation);

    let t_rgb_to_cmyk = lcms2::Transform::new_flags_context(
        lcms2::GlobalContext::new(),
        &srgb_profile,
        lcms2::PixelFormat::RGBA_8,
        &cmyk_profile,
        lcms2::PixelFormat::CMYK_8,
        lcms_intent,
        forward_flags,
    )
    .map_err(|e| {
        format!(
            "RGBからCMYKへの変換トランスフォーム作成に失敗しました: {}",
            e
        )
    })?;

    let pixel_count = (width as usize) * (height as usize);
    let mut cmyk_pixels = vec![0u8; pixel_count * 4];

    let parallelism = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    if parallelism > 1 && pixel_count >= 32_768 {
        let pixels_per_worker = pixel_count.div_ceil(parallelism);
        let bytes_per_worker = pixels_per_worker * 4;
        std::thread::scope(|s| {
            for (src_chunk, dst_chunk) in rgba_pixels
                .chunks(bytes_per_worker)
                .zip(cmyk_pixels.chunks_mut(bytes_per_worker))
            {
                let transform = &t_rgb_to_cmyk;
                s.spawn(move || {
                    transform.transform_pixels(src_chunk, dst_chunk);
                });
            }
        });
    } else {
        t_rgb_to_cmyk.transform_pixels(&rgba_pixels, &mut cmyk_pixels);
    }

    // Call PSD writer from psd.rs (executes planar sorting and Adobe ink density inversion)
    crate::psd::write_cmyk_psd(width, height, &cmyk_pixels, &icc_data, output_path)
        .map_err(|e| format!("CMYK PSDファイルの書き込みに失敗しました: {}", e))?;

    Ok(())
}

/// Direct copy of the raw RGB pixels, skipping ICC conversions and packaging into flat RGB PSD structure.
pub fn export_rgb_psd_file(input_path: &str, output_path: &str) -> Result<(), String> {
    let img = image::open(input_path)
        .map_err(|e| format!("元画像 '{}' の読み込みに失敗しました: {}", input_path, e))?;
    let width = img.width();
    let height = img.height();
    let rgba_pixels = img.to_rgba8().into_raw();

    // Writes raw RGB channels to planar layout, dropping Alpha
    crate::psd::write_rgb_psd(width, height, &rgba_pixels, output_path)
        .map_err(|e| format!("RGB PSDファイルの書き込みに失敗しました: {}", e))?;

    Ok(())
}

/// OS-dependent helper function to show the output folder and auto-select the written PSD file.
pub fn open_folder(target_path: &str) {
    let path = Path::new(target_path);
    // Resolve relative pathings to standard absolute format
    let abs_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

    #[cfg(target_os = "windows")]
    {
        // Spawns explorer.exe using the select parameter to highlight the target file
        let path_str = abs_path.to_string_lossy();
        // Remove UNC formatting prefixes (e.g. \\?\) if present
        let clean_path = if path_str.starts_with(r"\\?\") {
            &path_str[4..]
        } else {
            &path_str
        };
        let _ = Command::new("explorer.exe")
            .arg(format!("/select,{}", clean_path))
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        // Macintosh Finder reveal command
        let _ = Command::new("open").arg("-R").arg(abs_path).spawn();
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        // Standard Linux environments: opens the parent folder in the default file manager
        if let Some(parent) = abs_path.parent() {
            let _ = Command::new("xdg-open").arg(parent).spawn();
        }
    }
}
