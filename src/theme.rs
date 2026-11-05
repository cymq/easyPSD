use egui::{Color32, FontData, FontDefinitions, FontFamily, Stroke, Vec2, Visuals};

/// Shared semantic palette for easyPSD widgets, surfaces, and status messages.
pub struct AppColors {
    /// The primary royal blue brand color used for logos and core brand highlights.
    pub logo: Color32,
    /// Emerald green status color designed with high contrast for readability against light backgrounds.
    pub success_text: Color32,
    /// Rich red error color designed to satisfy contrast requirements on light canvases.
    pub error_text: Color32,
    pub success_bg: Color32,
    pub success_border: Color32,
    pub error_bg: Color32,
    pub error_border: Color32,
    pub text: Color32,
    pub border: Color32,
    pub inactive_bg: Color32,
    pub inactive_text: Color32,
    pub hover_bg: Color32,
    /// Shared white for card/window surfaces and text on accent backgrounds.
    pub surface: Color32,
    pub panel_bg: Color32,
    pub field_bg: Color32,
}

/// Existing theme and status colors, centralized without changing their values.
pub const COLORS: AppColors = AppColors {
    logo: Color32::from_rgb(0x41, 0x69, 0xE1), // Royal Blue brand color
    success_text: Color32::from_rgb(0x05, 0x96, 0x69), // Emerald green for success feedback
    error_text: Color32::from_rgb(0xDC, 0x26, 0x26), // Warning/Error red for error logs and alerts
    success_bg: Color32::from_rgb(0xEC, 0xFD, 0xF5),
    success_border: Color32::from_rgb(0xA7, 0xF3, 0xD0),
    error_bg: Color32::from_rgb(0xFE, 0xF2, 0xF2),
    error_border: Color32::from_rgb(0xFE, 0xCA, 0xCA),
    text: Color32::from_rgb(0x1F, 0x29, 0x37),
    border: Color32::from_rgb(0xE5, 0xE7, 0xEB),
    inactive_bg: Color32::from_rgb(0xF3, 0xF4, 0xF6),
    inactive_text: Color32::from_rgb(0x4B, 0x55, 0x63),
    hover_bg: Color32::from_rgb(0xEB, 0xF2, 0xFF),
    surface: Color32::WHITE,
    panel_bg: Color32::from_rgb(0xF5, 0xF6, 0xF8),
    field_bg: Color32::from_rgb(0xFA, 0xFA, 0xFC),
};

/// Shared layout dimensions in egui logical points; does not alter default spacing.
pub struct UiMetrics {
    pub header_padding: Vec2,
    pub header_logo_gap: f32,
    pub content_padding: f32,
    pub pane_gap: f32,
    pub min_content_size: f32,
    pub pane_padding: f32,
    pub pane_rounding: f32,
    pub card_padding: f32,
    pub card_rounding: f32,
    pub card_spacing: f32,
    pub inline_gap: f32,
    pub label_gap: f32,
    pub select_button_size: Vec2,
    pub intent_button_height: f32,
    pub export_button_height: f32,
    pub status_padding: f32,
    pub footer_height: f32,
    pub min_scroll_height: f32,
    pub field_rounding: f32,
    pub field_padding: Vec2,
    pub border_width: f32,
    pub emphasis_stroke_width: f32,
    pub adjustment_row_gap: f32,
    pub reset_button_size: Vec2,
    pub spinner_gap: f32,
    pub adjustment_label_width: f32,
    pub adjustment_value_width: f32,
    pub adjustment_control_height: f32,
    pub min_slider_width: f32,
    pub adjustment_drag_speed: f32,
    pub preview_padding: f32,
    pub preview_hint_height: f32,
    pub preview_hint_gap: f32,
    pub window_size: Vec2,
    pub min_window_size: Vec2,
}

pub const METRICS: UiMetrics = UiMetrics {
    header_padding: Vec2::new(24.0, 12.0),
    header_logo_gap: 8.0,
    content_padding: 12.0,
    pane_gap: 12.0,
    min_content_size: 10.0,
    pane_padding: 10.0,
    pane_rounding: 8.0,
    card_padding: 8.0,
    card_rounding: 6.0,
    card_spacing: 10.0,
    inline_gap: 6.0,
    label_gap: 2.0,
    select_button_size: Vec2::new(64.0, 26.0),
    intent_button_height: 30.0,
    export_button_height: 36.0,
    status_padding: 6.0,
    footer_height: 104.0,
    min_scroll_height: 60.0,
    field_rounding: 4.0,
    field_padding: Vec2::new(6.0, 3.0),
    border_width: 1.0,
    emphasis_stroke_width: 1.5,
    adjustment_row_gap: 4.0,
    reset_button_size: Vec2::new(58.0, 20.0),
    spinner_gap: 4.0,
    adjustment_label_width: 74.0,
    adjustment_value_width: 50.0,
    adjustment_control_height: 20.0,
    min_slider_width: 40.0,
    adjustment_drag_speed: 0.5,
    preview_padding: 4.0,
    preview_hint_height: 20.0,
    preview_hint_gap: 6.0,
    window_size: Vec2::new(1080.0, 740.0),
    min_window_size: Vec2::new(860.0, 560.0),
};

/// Explicit UI font sizes and line heights; leaves egui text styles unchanged.
pub struct Typography {
    pub logo: f32,
    pub version: f32,
    pub heading: f32,
    pub label: f32,
    pub field: f32,
    pub description: f32,
    pub hint: f32,
    pub export: f32,
    pub empty_line_height: f32,
    pub preview_line_height: f32,
}

pub const TYPOGRAPHY: Typography = Typography {
    logo: 22.0,
    version: 12.0,
    heading: 13.0,
    label: 11.5,
    field: 11.0,
    description: 10.5,
    hint: 10.0,
    export: 12.5,
    empty_line_height: 20.0,
    preview_line_height: 18.0,
};

/// Registers the embedded Japanese TTF font in the egui rendering context.
///
/// Inserts the font data as "GenInterfaceJP" and places it at index 0 of both the
/// Proportional and Monospace families to enforce proper CJK default rendering.
pub fn setup_fonts(ctx: &egui::Context, font_bytes: &'static [u8]) {
    let mut fonts = FontDefinitions::default();

    // Register the raw font byte array
    fonts.font_data.insert(
        "GenInterfaceJP".to_owned(),
        FontData::from_static(font_bytes),
    );

    // Set as the primary proportional (UI/Label) font
    if let Some(family) = fonts.families.get_mut(&FontFamily::Proportional) {
        family.insert(0, "GenInterfaceJP".to_owned());
    }
    // Set as the primary monospace (Paths/Status values) font
    if let Some(family) = fonts.families.get_mut(&FontFamily::Monospace) {
        family.insert(0, "GenInterfaceJP".to_owned());
    }

    // Apply the configured font definitions back to the context
    ctx.set_fonts(fonts);
}

/// Applies a premium, modern light theme to the egui Context.
///
/// Utilizes a soft gray background, custom rounded corners (6px to 8px), and
/// tailored color definitions for interactive button states.
pub fn apply_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    // Use egui's default light visuals as our base template
    let mut visuals = Visuals::light();

    // Configure premium rounded corners for modern card-like appearance
    visuals.window_rounding = METRICS.pane_rounding.into(); // Outer dialogs and main workspace panels
    visuals.menu_rounding = METRICS.card_rounding.into(); // Dropdowns and context menus
    visuals.widgets.noninteractive.rounding = METRICS.pane_rounding.into(); // Static panels and group cards
    visuals.widgets.inactive.rounding = METRICS.card_rounding.into(); // Default state buttons
    visuals.widgets.hovered.rounding = METRICS.card_rounding.into(); // Hover state elements
    visuals.widgets.active.rounding = METRICS.card_rounding.into(); // Clicked/pressed elements

    // Apply hyperlink colors
    visuals.hyperlink_color = COLORS.logo;

    // State 1: Inactive / Idle elements (e.g. unselected buttons, text fields)
    visuals.widgets.inactive.bg_fill = COLORS.inactive_bg;
    visuals.widgets.inactive.fg_stroke = Stroke::new(METRICS.border_width, COLORS.inactive_text);
    visuals.widgets.inactive.bg_stroke = Stroke::new(METRICS.border_width, COLORS.border);

    // State 2: Hovered elements (subtle blue-tinted visual cue when cursor enters)
    visuals.widgets.hovered.bg_fill = COLORS.hover_bg;
    visuals.widgets.hovered.fg_stroke = Stroke::new(METRICS.emphasis_stroke_width, COLORS.logo);
    visuals.widgets.hovered.bg_stroke = Stroke::new(METRICS.border_width, COLORS.logo);

    // State 3: Active / Pressed elements (solid brand-filled highlight)
    visuals.widgets.active.bg_fill = COLORS.logo;
    visuals.widgets.active.fg_stroke = Stroke::new(METRICS.emphasis_stroke_width, COLORS.surface);
    visuals.widgets.active.bg_stroke = Stroke::new(METRICS.border_width, COLORS.logo);

    // State 4: Static / Non-interactive elements (such as container panels and cards)
    visuals.widgets.noninteractive.bg_fill = COLORS.surface;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(METRICS.border_width, COLORS.border);

    // Configure checkmark indicators and slider filling fills
    visuals.selection.bg_fill = COLORS.logo;
    visuals.selection.stroke = Stroke::new(METRICS.border_width, COLORS.surface);

    // Panel background properties and extreme inset spaces
    visuals.panel_fill = COLORS.panel_bg;
    visuals.window_fill = COLORS.surface;
    visuals.extreme_bg_color = COLORS.field_bg;

    // Set the overall text color override to Slate Gray for maximum readability
    visuals.override_text_color = Some(COLORS.text);

    // Commit custom visuals back to the egui Context
    style.visuals = visuals;
    ctx.set_style(style);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_theme_preserves_existing_colors() {
        let ctx = egui::Context::default();
        apply_theme(&ctx);
        let style = ctx.style();
        let visuals = &style.visuals;
        let blue = Color32::from_rgb(0x41, 0x69, 0xE1);
        let white = Color32::WHITE;
        let border = Color32::from_rgb(0xE5, 0xE7, 0xEB);

        assert_eq!(COLORS.logo, blue);
        assert_eq!(COLORS.success_text, Color32::from_rgb(0x05, 0x96, 0x69));
        assert_eq!(COLORS.error_text, Color32::from_rgb(0xDC, 0x26, 0x26));
        assert_eq!(COLORS.success_bg, Color32::from_rgb(0xEC, 0xFD, 0xF5));
        assert_eq!(COLORS.success_border, Color32::from_rgb(0xA7, 0xF3, 0xD0));
        assert_eq!(COLORS.error_bg, Color32::from_rgb(0xFE, 0xF2, 0xF2));
        assert_eq!(COLORS.error_border, Color32::from_rgb(0xFE, 0xCA, 0xCA));
        assert_eq!(visuals.hyperlink_color, blue);
        assert_eq!(
            visuals.widgets.inactive.bg_fill,
            Color32::from_rgb(0xF3, 0xF4, 0xF6)
        );
        assert_eq!(
            visuals.widgets.inactive.fg_stroke.color,
            Color32::from_rgb(0x4B, 0x55, 0x63)
        );
        assert_eq!(visuals.widgets.inactive.bg_stroke.color, border);
        assert_eq!(
            visuals.widgets.hovered.bg_fill,
            Color32::from_rgb(0xEB, 0xF2, 0xFF)
        );
        assert_eq!(visuals.widgets.hovered.fg_stroke.color, blue);
        assert_eq!(visuals.widgets.hovered.bg_stroke.color, blue);
        assert_eq!(visuals.widgets.active.bg_fill, blue);
        assert_eq!(visuals.widgets.active.fg_stroke.color, white);
        assert_eq!(visuals.widgets.active.bg_stroke.color, blue);
        assert_eq!(visuals.widgets.noninteractive.bg_fill, white);
        assert_eq!(visuals.widgets.noninteractive.bg_stroke.color, border);
        assert_eq!(visuals.selection.bg_fill, blue);
        assert_eq!(visuals.selection.stroke.color, white);
        assert_eq!(visuals.panel_fill, Color32::from_rgb(0xF5, 0xF6, 0xF8));
        assert_eq!(visuals.window_fill, white);
        assert_eq!(
            visuals.extreme_bg_color,
            Color32::from_rgb(0xFA, 0xFA, 0xFC)
        );
        assert_eq!(
            visuals.override_text_color,
            Some(Color32::from_rgb(0x1F, 0x29, 0x37))
        );
    }

    #[test]
    fn apply_theme_preserves_rounding_and_uses_metric_stroke_widths() {
        let ctx = egui::Context::default();
        apply_theme(&ctx);
        let style = ctx.style();
        let visuals = &style.visuals;

        assert_eq!(METRICS.pane_rounding, 8.0);
        assert_eq!(METRICS.card_rounding, 6.0);
        assert_eq!(visuals.window_rounding, egui::Rounding::same(8.0));
        assert_eq!(visuals.menu_rounding, egui::Rounding::same(6.0));
        assert_eq!(
            visuals.widgets.noninteractive.rounding,
            egui::Rounding::same(8.0)
        );
        for widget in [
            &visuals.widgets.inactive,
            &visuals.widgets.hovered,
            &visuals.widgets.active,
        ] {
            assert_eq!(widget.rounding, egui::Rounding::same(6.0));
        }

        assert_eq!(METRICS.border_width, 1.0);
        assert_eq!(METRICS.emphasis_stroke_width, 1.5);
        for stroke in [
            visuals.widgets.inactive.fg_stroke,
            visuals.widgets.inactive.bg_stroke,
            visuals.widgets.hovered.bg_stroke,
            visuals.widgets.active.bg_stroke,
            visuals.widgets.noninteractive.bg_stroke,
            visuals.selection.stroke,
        ] {
            assert_eq!(stroke.width, METRICS.border_width);
        }
        for stroke in [
            visuals.widgets.hovered.fg_stroke,
            visuals.widgets.active.fg_stroke,
        ] {
            assert_eq!(stroke.width, METRICS.emphasis_stroke_width);
        }
    }

    #[test]
    fn apply_theme_preserves_text_styles_and_custom_spacing() {
        let ctx = egui::Context::default();
        let mut before = (*ctx.style()).clone();
        before.spacing.item_spacing = Vec2::new(17.0, 19.0);
        before.spacing.button_padding = Vec2::new(13.0, 7.0);
        before
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(17.0));
        ctx.set_style(before.clone());

        apply_theme(&ctx);

        let after = ctx.style();
        assert_eq!(after.text_styles, before.text_styles);
        assert_eq!(after.spacing.item_spacing, before.spacing.item_spacing);
        assert_eq!(after.spacing.button_padding, before.spacing.button_padding);
    }
}
