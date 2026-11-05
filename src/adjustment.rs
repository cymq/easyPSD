//! Pre-CMYK sRGB image adjustment engine (local Shadows/Highlights, midtone contrast, and HSV).

use apeireth_image_algo::tone::{ShadowsHighlights, shadows_highlights};
use palette::{Darken, Desaturate, FromColor, Hsv, Lighten, ShiftHue, Srgb};

/// User-configurable image adjustment parameters applied in sRGB space
/// immediately prior to ICC CMYK conversion.
///
/// Performing adjustments before forward CMM translation ensures that
/// target ICC profile constraints (Total Ink Limit / TAC and GCR/UCR black
/// generation) remain strictly preserved in the final CMYK output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct AdjustmentParams {
    /// Hue shift in degrees: -90 to +90 (0 = neutral).
    pub hue: i32,
    /// Saturation adjustment: -100 to +100 (0 = neutral).
    pub saturation: i32,
    /// Value (brightness) adjustment: -100 to +100 (0 = neutral).
    pub value: i32,
    /// Contrast adjustment: -100 to +100 (0 = neutral).
    pub contrast: i32,
    /// Highlights tonal adjustment: -100 to +100 (0 = neutral).
    pub highlights: i32,
    /// Shadows tonal adjustment: -100 to +100 (0 = neutral).
    pub shadows: i32,
}

impl AdjustmentParams {
    pub const HUE_MIN: i32 = -90;
    pub const HUE_MAX: i32 = 90;
    pub const PARAM_MIN: i32 = -100;
    pub const PARAM_MAX: i32 = 100;

    /// Returns true if all adjustment sliders are at their neutral `0` position.
    #[inline]
    pub fn is_identity(&self) -> bool {
        self.hue == 0
            && self.saturation == 0
            && self.value == 0
            && self.contrast == 0
            && self.highlights == 0
            && self.shadows == 0
    }

    /// Resets all parameters back to `0`.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    fn clamped(self) -> Self {
        Self {
            hue: self.hue.clamp(Self::HUE_MIN, Self::HUE_MAX),
            saturation: self.saturation.clamp(Self::PARAM_MIN, Self::PARAM_MAX),
            value: self.value.clamp(Self::PARAM_MIN, Self::PARAM_MAX),
            contrast: self.contrast.clamp(Self::PARAM_MIN, Self::PARAM_MAX),
            highlights: self.highlights.clamp(Self::PARAM_MIN, Self::PARAM_MAX),
            shadows: self.shadows.clamp(Self::PARAM_MIN, Self::PARAM_MAX),
        }
    }
}

const REFERENCE_IMAGE_LONG_EDGE: f32 = 1600.0;
const REFERENCE_TONE_RADIUS: f32 = 30.0;
const SATURATION_EPSILON: f32 = 1e-6;
const MAX_SATURATION_BOOST_GAIN: f32 = 2.0;

/// Smoothly boosts HSV saturation `s` in `[0.0, 1.0]` by `amount` in `[0.0, 1.0]`
/// using a Schlick rational curve: `S' = ((1 + k) * S) / (1 + k * S)` where `k = 2 * amount`.
///
/// Unlike linear headroom interpolation (`S + (1 - S) * amount`), which forces near-neutral
/// pixels (`S ~ 0.01`) to jump immediately to `amount`, this curve scales low-saturation
/// regions gently in proportion to their existing chroma while smoothly compressing toward `1.0`.
#[inline]
fn boost_hsv_saturation(s: f32, amount: f32) -> f32 {
    let k = MAX_SATURATION_BOOST_GAIN * amount.clamp(0.0, 1.0);
    ((1.0 + k) * s / (1.0 + k * s)).clamp(0.0, 1.0)
}

/// Applies local Shadows/Highlights and midtone contrast, followed by HSV
/// adjustments, to an interleaved 8-bit sRGB RGBA image.
///
/// Alpha bytes and fully transparent pixels (including their RGB) are preserved
/// exactly. Empty images and neutral parameters return an exact copy.
///
/// # Panics
///
/// Panics if `width * height * 4` overflows or differs from `rgba.len()`, even
/// when the image is empty or the parameters are neutral.
pub fn apply_adjustments(
    rgba: &[u8],
    width: usize,
    height: usize,
    params: AdjustmentParams,
) -> Vec<u8> {
    let expected_len = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(4))
        .expect("RGBA image dimensions overflow usize");
    assert_eq!(
        rgba.len(),
        expected_len,
        "RGBA buffer length must equal width * height * 4"
    );

    let params = params.clamped();
    if rgba.is_empty() || params.is_identity() {
        return rgba.to_vec();
    }

    let mut pixels: Vec<[f32; 4]> = rgba
        .chunks_exact(4)
        .map(|p| {
            [
                p[0] as f32 / 255.0,
                p[1] as f32 / 255.0,
                p[2] as f32 / 255.0,
                p[3] as f32 / 255.0,
            ]
        })
        .collect();

    let reverse = tone_pass_params(params, width, height, true);
    if apply_tone_pass(&mut pixels, width, height, &reverse) {
        // Reflect the upstream delta about the original RGB, without changing
        // its spatial/tonal mask. This first pass can use the source bytes as
        // its baseline, avoiding another float buffer. Reflected darkening can
        // clip shadows to 0; reflected brightening can likewise clip to 1.
        for (pixel, original) in pixels.iter_mut().zip(rgba.chunks_exact(4)) {
            for c in 0..3 {
                pixel[c] = (2.0 * original[c] as f32 / 255.0 - pixel[c]).clamp(0.0, 1.0);
            }
        }
    }
    let normal = tone_pass_params(params, width, height, false);
    apply_tone_pass(&mut pixels, width, height, &normal);

    let has_hsv = params.hue != 0 || params.saturation != 0 || params.value != 0;
    let saturation = params.saturation as f32 / 100.0;
    let value = params.value as f32 / 100.0;
    let mut out = rgba.to_vec();
    for (pixel, dst) in pixels.iter().zip(out.chunks_exact_mut(4)) {
        if dst[3] == 0 {
            continue;
        }
        let mut rgb = Srgb::<f32>::new(pixel[0], pixel[1], pixel[2]);
        if has_hsv {
            let mut hsv = Hsv::from_color(rgb);
            if params.hue != 0 {
                hsv = hsv.shift_hue(params.hue as f32);
            }
            if saturation > 0.0 && hsv.saturation > SATURATION_EPSILON {
                hsv.saturation = boost_hsv_saturation(hsv.saturation, saturation);
            } else if saturation < 0.0 {
                hsv = hsv.desaturate(-saturation);
            }
            if value > 0.0 {
                hsv = hsv.lighten(value);
            } else if value < 0.0 {
                hsv = hsv.darken(-value);
            }
            rgb = Srgb::from_color(hsv);
        }
        dst[0] = to_u8(rgb.red);
        dst[1] = to_u8(rgb.green);
        dst[2] = to_u8(rgb.blue);
    }
    out
}

fn tone_radius(width: usize, height: usize) -> f32 {
    REFERENCE_TONE_RADIUS * width.max(height) as f32 / REFERENCE_IMAGE_LONG_EDGE
}

fn tone_pass_params(
    params: AdjustmentParams,
    width: usize,
    height: usize,
    reverse: bool,
) -> ShadowsHighlights {
    let (shadow_amount, highlight_amount, midtone) = if reverse {
        (
            (-params.shadows).max(0) as f32,
            params.highlights.max(0) as f32,
            0.0,
        )
    } else {
        (
            params.shadows.max(0) as f32,
            (-params.highlights).max(0) as f32,
            params.contrast as f32,
        )
    };
    let radius = tone_radius(width, height);
    ShadowsHighlights {
        shadow_amount,
        shadow_tone: 50.0,
        shadow_radius: radius,
        highlight_amount,
        highlight_tone: 50.0,
        highlight_radius: radius,
        color: 0.0,
        midtone,
        black_clip: 0.0,
        white_clip: 0.0,
    }
}

fn apply_tone_pass(
    pixels: &mut [[f32; 4]],
    width: usize,
    height: usize,
    params: &ShadowsHighlights,
) -> bool {
    if params.shadow_amount == 0.0 && params.highlight_amount == 0.0 && params.midtone == 0.0 {
        return false;
    }
    // Neighbourhood-dependent correction must see the full image, not chunks.
    shadows_highlights(pixels, width, height, params);
    true
}

fn to_u8(channel: f32) -> u8 {
    (channel.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gray_patch(gray: u8, width: usize, height: usize) -> Vec<u8> {
        [gray, gray, gray, 255].repeat(width * height)
    }

    #[test]
    fn identity_is_exact_noop() {
        let src: Vec<u8> = (0..=255_u8)
            .flat_map(|v| [v, 255 - v, v.wrapping_mul(13), v])
            .collect();
        assert_eq!(apply_adjustments(&src, 16, 16, Default::default()), src);
    }

    #[test]
    fn reset_and_public_limits_are_preserved() {
        assert_eq!(
            (AdjustmentParams::HUE_MIN, AdjustmentParams::HUE_MAX),
            (-90, 90)
        );
        assert_eq!(
            (AdjustmentParams::PARAM_MIN, AdjustmentParams::PARAM_MAX),
            (-100, 100)
        );
        let mut params = AdjustmentParams {
            hue: 20,
            saturation: -30,
            value: 40,
            contrast: -50,
            highlights: 60,
            shadows: -70,
        };
        assert!(!params.is_identity());
        params.reset();
        assert!(params.is_identity());
        assert_eq!(params, AdjustmentParams::default());
    }

    #[test]
    fn empty_images_are_exact_noops() {
        let params = AdjustmentParams {
            shadows: 100,
            hue: 90,
            ..Default::default()
        };
        for (width, height) in [(0, 0), (0, 7), (9, 0), (usize::MAX, 0)] {
            assert!(apply_adjustments(&[], width, height, params).is_empty());
        }
    }

    #[test]
    fn shape_mismatches_panic_even_for_neutral_or_empty_input() {
        for (src, width, height) in [
            (vec![1, 2, 3], 1, 1),
            (vec![0; 8], 1, 1),
            (vec![0; 4], 0, 1),
            (vec![], 1, 1),
        ] {
            assert!(
                std::panic::catch_unwind(|| {
                    apply_adjustments(&src, width, height, Default::default())
                })
                .is_err()
            );
        }
    }

    #[test]
    #[should_panic(expected = "RGBA image dimensions overflow usize")]
    fn pixel_count_overflow_panics() {
        apply_adjustments(&[], usize::MAX, 2, Default::default());
    }

    #[test]
    #[should_panic(expected = "RGBA image dimensions overflow usize")]
    fn byte_count_overflow_panics() {
        apply_adjustments(&[], usize::MAX / 4 + 1, 1, Default::default());
    }

    #[test]
    fn out_of_range_sliders_are_clamped_before_mapping() {
        let src = [35, 80, 120, 255, 220, 170, 90, 128, 12, 34, 56, 0];
        for sign in [-1, 1] {
            let extreme = if sign < 0 { i32::MIN } else { i32::MAX };
            let params = AdjustmentParams {
                hue: extreme,
                saturation: extreme,
                value: extreme,
                contrast: extreme,
                highlights: extreme,
                shadows: extreme,
            };
            let bounded = AdjustmentParams {
                hue: sign * 90,
                saturation: sign * 100,
                value: sign * 100,
                contrast: sign * 100,
                highlights: sign * 100,
                shadows: sign * 100,
            };
            assert_eq!(params.clamped(), bounded);
            assert_eq!(
                apply_adjustments(&src, 3, 1, params),
                apply_adjustments(&src, 3, 1, bounded)
            );
        }
    }

    #[test]
    fn every_alpha_byte_is_preserved() {
        let src: Vec<u8> = (0..=255_u8).flat_map(|a| [40, 110, 210, a]).collect();
        for (shadows, highlights) in [(70, -50), (-70, 50)] {
            let out = apply_adjustments(
                &src,
                16,
                16,
                AdjustmentParams {
                    hue: 35,
                    saturation: 50,
                    value: -20,
                    contrast: 30,
                    shadows,
                    highlights,
                },
            );
            for (input, output) in src.chunks_exact(4).zip(out.chunks_exact(4)) {
                assert_eq!(output[3], input[3]);
                if input[3] == 0 {
                    assert_eq!(output, input);
                }
            }
        }
    }

    #[test]
    fn transparent_rgba_is_untouched_for_both_passes_and_hsv() {
        let src = [12, 34, 56, 0, 255, 0, 128, 0, 80, 80, 80, 255];
        for (shadows, highlights) in [(75, 60), (-75, -60)] {
            let out = apply_adjustments(
                &src,
                3,
                1,
                AdjustmentParams {
                    hue: 90,
                    saturation: 100,
                    value: 100,
                    contrast: 100,
                    shadows,
                    highlights,
                },
            );
            assert_eq!(&out[..8], &src[..8]);
        }
    }

    #[test]
    fn gray_stays_achromatic() {
        let src: Vec<u8> = [0, 32, 64, 128, 192, 224, 255]
            .into_iter()
            .flat_map(|v| [v, v, v, 255])
            .collect();
        for (shadows, highlights) in [(50, -40), (-50, 40)] {
            let out = apply_adjustments(
                &src,
                7,
                1,
                AdjustmentParams {
                    hue: 45,
                    saturation: 100,
                    value: 20,
                    contrast: 35,
                    shadows,
                    highlights,
                },
            );
            for px in out.chunks_exact(4) {
                assert_eq!(px[0], px[1]);
                assert_eq!(px[1], px[2]);
            }
        }
        // Saturation alone must not introduce colour into even black pixels.
        assert_eq!(
            apply_adjustments(
                &src,
                7,
                1,
                AdjustmentParams {
                    saturation: 100,
                    ..Default::default()
                }
            ),
            src
        );
    }

    #[test]
    fn signed_shadows_and_highlights_move_flat_patches_in_expected_directions() {
        for amount in [-75, 75] {
            let dark = gray_patch(40, 8, 8);
            let out = apply_adjustments(
                &dark,
                8,
                8,
                AdjustmentParams {
                    shadows: amount,
                    ..Default::default()
                },
            );
            for px in out.chunks_exact(4) {
                assert!(
                    (px[0] as i32 - 40) * amount > 0,
                    "shadows amount={amount}, input=40, actual pixel={px:?}"
                );
            }
            let bright = gray_patch(215, 8, 8);
            let out = apply_adjustments(
                &bright,
                8,
                8,
                AdjustmentParams {
                    highlights: amount,
                    ..Default::default()
                },
            );
            for px in out.chunks_exact(4) {
                assert!(
                    (px[0] as i32 - 215) * amount > 0,
                    "highlights amount={amount}, input=215, actual pixel={px:?}"
                );
            }
        }
    }

    #[test]
    fn reflected_delta_clips_extreme_shadows_and_highlights() {
        let dark = gray_patch(32, 8, 8);
        let out = apply_adjustments(
            &dark,
            8,
            8,
            AdjustmentParams {
                shadows: -100,
                ..Default::default()
            },
        );
        assert_eq!(out, gray_patch(0, 8, 8));

        let bright = gray_patch(223, 8, 8);
        let out = apply_adjustments(
            &bright,
            8,
            8,
            AdjustmentParams {
                highlights: 100,
                ..Default::default()
            },
        );
        assert_eq!(out, gray_patch(255, 8, 8));
    }

    #[test]
    fn hue_shift_90_degrees() {
        let out = apply_adjustments(
            &[255, 0, 0, 255],
            1,
            1,
            AdjustmentParams {
                hue: 90,
                ..Default::default()
            },
        );
        assert!((out[0] as i32 - 128).abs() <= 1);
        assert_eq!(&out[1..], &[255, 0, 255]);
    }

    #[test]
    fn desaturation_to_minus_100_uses_hsv_value() {
        let out = apply_adjustments(
            &[240, 40, 120, 77],
            1,
            1,
            AdjustmentParams {
                saturation: -100,
                ..Default::default()
            },
        );
        assert_eq!(out, [240, 240, 240, 77]);
    }

    #[test]
    fn positive_saturation_scales_near_neutral_pixels_gently_and_mid_tones_monotonically() {
        // Near-white pixel with ~2% saturation: [250, 245, 245, 255]
        let near_white = [250u8, 245, 245, 255];
        let out_plus_50 = apply_adjustments(
            &near_white,
            1,
            1,
            AdjustmentParams {
                saturation: 50,
                ..Default::default()
            },
        );
        let out_plus_100 = apply_adjustments(
            &near_white,
            1,
            1,
            AdjustmentParams {
                saturation: 100,
                ..Default::default()
            },
        );
        // Previously, +50 dropped green/blue from 245 down to ~125; now they stay >= 230
        assert!(out_plus_50[1] >= 238 && out_plus_50[1] < 245);
        assert!(out_plus_100[1] >= 233 && out_plus_100[1] < out_plus_50[1]);

        // Moderately saturated pixel: [200, 100, 100, 255] (S = 0.5)
        let mid_sat = [200u8, 100, 100, 255];
        let mid_50 = apply_adjustments(
            &mid_sat,
            1,
            1,
            AdjustmentParams {
                saturation: 50,
                ..Default::default()
            },
        );
        let mid_100 = apply_adjustments(
            &mid_sat,
            1,
            1,
            AdjustmentParams {
                saturation: 100,
                ..Default::default()
            },
        );
        assert!(mid_50[1] < 100 && mid_100[1] < mid_50[1]);
    }

    #[test]
    fn signed_value_moves_brightness_in_expected_direction() {
        for value in [-50, 50] {
            let out = apply_adjustments(
                &[80, 120, 160, 255],
                1,
                1,
                AdjustmentParams {
                    value,
                    ..Default::default()
                },
            );
            assert!((out[2] as i32 - 160) * value > 0);
        }
        assert_eq!(
            apply_adjustments(
                &[80, 120, 160, 255],
                1,
                1,
                AdjustmentParams {
                    value: -100,
                    ..Default::default()
                }
            ),
            [0, 0, 0, 255]
        );
        assert_eq!(
            apply_adjustments(
                &[80, 120, 160, 255],
                1,
                1,
                AdjustmentParams {
                    value: 100,
                    ..Default::default()
                }
            )[2],
            255
        );
    }

    #[test]
    fn contrast_expands_or_compresses_around_half() {
        let src = [64, 64, 64, 255, 192, 192, 192, 255];
        for contrast in [-75, 75] {
            let out = apply_adjustments(
                &src,
                2,
                1,
                AdjustmentParams {
                    contrast,
                    ..Default::default()
                },
            );
            assert!((out[0] as i32 - 64) * contrast < 0);
            assert!((out[4] as i32 - 192) * contrast > 0);
        }
    }

    #[test]
    fn small_thin_and_nonsquare_images_are_valid() {
        for (width, height) in [(1, 1), (1, 17), (17, 1), (3, 7), (7, 3)] {
            let src = gray_patch(96, width, height);
            let out = apply_adjustments(
                &src,
                width,
                height,
                AdjustmentParams {
                    shadows: 50,
                    highlights: 30,
                    contrast: 20,
                    saturation: 10,
                    ..Default::default()
                },
            );
            assert_eq!(out.len(), src.len());
            assert!(out.chunks_exact(4).all(|p| p[3] == 255));
        }
    }

    #[test]
    fn radius_tracks_long_edge_without_large_image_clamp() {
        for (width, height, expected) in [
            (1600, 900, 30.0),
            (900, 1600, 30.0),
            (800, 450, 15.0),
            (400, 225, 7.5),
            (16000, 9000, 300.0),
        ] {
            assert_eq!(tone_radius(width, height), expected);
            let p = tone_pass_params(Default::default(), width, height, false);
            assert_eq!(p.shadow_radius, expected);
            assert_eq!(p.highlight_radius, expected);
        }
        assert!((tone_radius(1, 1) - 30.0 / 1600.0).abs() < f32::EPSILON);
    }

    #[test]
    fn pass_mapping_uses_percent_units_and_no_implicit_adjustments() {
        let params = AdjustmentParams {
            shadows: -65,
            highlights: 45,
            contrast: -35,
            ..Default::default()
        };
        let reverse = tone_pass_params(params, 800, 600, true);
        assert_eq!(
            (
                reverse.shadow_amount,
                reverse.highlight_amount,
                reverse.midtone
            ),
            (65.0, 45.0, 0.0)
        );
        let normal = tone_pass_params(params, 800, 600, false);
        assert_eq!(
            (
                normal.shadow_amount,
                normal.highlight_amount,
                normal.midtone
            ),
            (0.0, 0.0, -35.0)
        );
        let params = AdjustmentParams {
            shadows: 65,
            highlights: -45,
            contrast: 35,
            ..Default::default()
        };
        let normal = tone_pass_params(params, 800, 600, false);
        assert_eq!(
            (
                normal.shadow_amount,
                normal.highlight_amount,
                normal.midtone
            ),
            (65.0, 45.0, 35.0)
        );
        for reverse in [false, true] {
            let p = tone_pass_params(Default::default(), 800, 600, reverse);
            assert_eq!(
                (p.shadow_amount, p.highlight_amount, p.midtone),
                (0.0, 0.0, 0.0)
            );
            assert_eq!((p.shadow_tone, p.highlight_tone), (50.0, 50.0));
            assert_eq!((p.color, p.black_clip, p.white_clip), (0.0, 0.0, 0.0));
        }
    }

    #[test]
    fn inactive_pass_is_an_exact_float_noop() {
        let original = [[0.123, 0.456, 0.789, 0.5], [0.9, 0.2, 0.6, 0.0]];
        let mut pixels = original;
        for reverse in [true, false] {
            let p = tone_pass_params(Default::default(), 2, 1, reverse);
            assert!(!apply_tone_pass(&mut pixels, 2, 1, &p));
            assert_eq!(pixels, original);
        }
    }

    // This reference deliberately constructs upstream parameters independently
    // of tone_pass_params, so unit/sign mapping and pass order are covered too.
    fn direct_upstream(
        src: &[u8],
        width: usize,
        height: usize,
        passes: &[(bool, f32, f32, f32)],
    ) -> Vec<u8> {
        let mut pixels: Vec<[f32; 4]> = src
            .chunks_exact(4)
            .map(|p| {
                [
                    p[0] as f32 / 255.0,
                    p[1] as f32 / 255.0,
                    p[2] as f32 / 255.0,
                    p[3] as f32 / 255.0,
                ]
            })
            .collect();
        for &(reverse, shadow_amount, highlight_amount, midtone) in passes {
            let radius = 30.0 * width.max(height) as f32 / 1600.0;
            shadows_highlights(
                &mut pixels,
                width,
                height,
                &ShadowsHighlights {
                    shadow_amount,
                    shadow_tone: 50.0,
                    shadow_radius: radius,
                    highlight_amount,
                    highlight_tone: 50.0,
                    highlight_radius: radius,
                    color: 0.0,
                    midtone,
                    black_clip: 0.0,
                    white_clip: 0.0,
                },
            );
            if reverse {
                for (pixel, original) in pixels.iter_mut().zip(src.chunks_exact(4)) {
                    for c in 0..3 {
                        pixel[c] = (2.0 * original[c] as f32 / 255.0 - pixel[c]).clamp(0.0, 1.0);
                    }
                }
            }
        }
        let mut expected = src.to_vec();
        for (pixel, dst) in pixels.iter().zip(expected.chunks_exact_mut(4)) {
            if dst[3] != 0 {
                for c in 0..3 {
                    dst[c] = (pixel[c].clamp(0.0, 1.0) * 255.0).round() as u8;
                }
            }
        }
        expected
    }

    #[test]
    fn normal_direction_matches_direct_upstream_on_full_nonsquare_image() {
        let (width, height) = (32, 17);
        let src: Vec<u8> = (0..width * height)
            .flat_map(|i| {
                [
                    (i * 29 % 256) as u8,
                    (i * 71 % 256) as u8,
                    (i * 13 % 256) as u8,
                    (i % 256) as u8,
                ]
            })
            .collect();
        for (shadows, highlights, contrast) in [(60, -45, 25), (60, 0, 0), (0, -45, 0), (0, 0, -35)]
        {
            let out = apply_adjustments(
                &src,
                width,
                height,
                AdjustmentParams {
                    shadows,
                    highlights,
                    contrast,
                    ..Default::default()
                },
            );
            let expected = direct_upstream(
                &src,
                width,
                height,
                &[(false, shadows as f32, -highlights as f32, contrast as f32)],
            );
            assert_eq!(out, expected);
        }
    }

    #[test]
    fn reverse_precedes_normal_and_contrast_matches_direct_upstream() {
        let (width, height) = (19, 23);
        let src: Vec<u8> = (0..width * height)
            .flat_map(|i| {
                [
                    (i * 17 % 256) as u8,
                    (i * 43 % 256) as u8,
                    (i * 97 % 256) as u8,
                    255,
                ]
            })
            .collect();
        for (
            shadows,
            highlights,
            reverse_shadow,
            reverse_highlight,
            normal_shadow,
            normal_highlight,
        ) in [
            (-55, 40, 55.0, 40.0, 0.0, 0.0),
            (-55, -40, 55.0, 0.0, 0.0, 40.0),
            (55, 40, 0.0, 40.0, 55.0, 0.0),
        ] {
            let out = apply_adjustments(
                &src,
                width,
                height,
                AdjustmentParams {
                    shadows,
                    highlights,
                    contrast: 30,
                    ..Default::default()
                },
            );
            let expected = direct_upstream(
                &src,
                width,
                height,
                &[
                    (true, reverse_shadow, reverse_highlight, 0.0),
                    (false, normal_shadow, normal_highlight, 30.0),
                ],
            );
            assert_eq!(out, expected);
        }
    }
}
