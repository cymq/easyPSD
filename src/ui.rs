use egui::{Color32, Frame, Response, RichText, Stroke, Style, Ui, Visuals};

use crate::conversion::RenderingIntent;
use crate::theme::{COLORS, METRICS, TYPOGRAPHY};

pub fn pane_frame(style: &Style) -> Frame {
    Frame::none()
        .fill(style.visuals.window_fill())
        .stroke(style.visuals.widgets.noninteractive.bg_stroke)
        .rounding(METRICS.pane_rounding)
        .inner_margin(METRICS.pane_padding)
}

pub fn card_frame(style: &Style) -> Frame {
    Frame::none()
        .fill(style.visuals.widgets.noninteractive.bg_fill)
        .stroke(style.visuals.widgets.noninteractive.bg_stroke)
        .rounding(METRICS.card_rounding)
        .inner_margin(METRICS.card_padding)
}

/// Draws the filename and picker button, leaving file dialog handling to the caller.
pub fn file_select_row(ui: &mut Ui, path: &str, placeholder: &str, width: f32) -> Response {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let filename = if path.is_empty() {
            placeholder.to_owned()
        } else {
            std::path::Path::new(path)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| "未選択".to_owned())
        };

        // Read context visuals, as before extraction, even inside disabled UI scopes.
        let style = ui.ctx().style();
        let visuals = &style.visuals;
        let field_width = (width - METRICS.select_button_size.x - METRICS.inline_gap)
            .max(METRICS.min_content_size);
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(field_width, METRICS.select_button_size.y),
            egui::Sense::hover(),
        );
        ui.painter().rect(
            rect,
            METRICS.field_rounding,
            visuals.extreme_bg_color,
            Stroke::new(
                METRICS.border_width,
                visuals.widgets.noninteractive.bg_stroke.color,
            ),
        );

        let mut field_ui = ui.child_ui(
            rect.shrink2(METRICS.field_padding),
            egui::Layout::left_to_right(egui::Align::Center),
        );
        let text_color = if path.is_empty() {
            visuals.weak_text_color()
        } else {
            visuals.text_color()
        };
        field_ui.add(
            egui::Label::new(
                RichText::new(filename)
                    .size(TYPOGRAPHY.field)
                    .color(text_color),
            )
            .truncate(true),
        );

        ui.add_space(METRICS.inline_gap);
        ui.add(
            egui::Button::new(RichText::new("選択...").size(TYPOGRAPHY.label))
                .min_size(METRICS.select_button_size),
        )
    })
    .inner
}

/// Keeps the existing two-column order and reports clicks, including on the active intent.
pub fn intent_buttons(ui: &mut Ui, active_index: &mut usize, width: f32) -> bool {
    let button_width = (width - METRICS.inline_gap) / 2.0;
    let mut clicked = false;
    for (row_index, row) in RenderingIntent::ALL.chunks(2).enumerate() {
        if row_index > 0 {
            ui.add_space(METRICS.inline_gap);
        }
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for (column, intent) in row.iter().enumerate() {
                if column > 0 {
                    ui.add_space(METRICS.inline_gap);
                }
                let style = ui.ctx().style();
                let visuals = &style.visuals;
                let (background, text) = if *active_index == intent.to_index() {
                    (visuals.selection.bg_fill, visuals.selection.stroke.color)
                } else {
                    (
                        visuals.widgets.inactive.bg_fill,
                        visuals.widgets.inactive.fg_stroke.color,
                    )
                };
                let button = egui::Button::new(
                    RichText::new(intent.label())
                        .size(TYPOGRAPHY.label)
                        .color(text),
                )
                .fill(background)
                .min_size(egui::vec2(button_width, METRICS.intent_button_height));
                if ui.add(button).clicked() {
                    *active_index = intent.to_index();
                    clicked = true;
                }
            }
        });
    }
    clicked
}

pub struct StatusStyle {
    pub background: Color32,
    pub stroke: Stroke,
    pub text: Color32,
}

/// Preserves the current message-based classification, with errors taking precedence.
pub fn status_style(status: &str, visuals: &Visuals) -> StatusStyle {
    if status.starts_with("エラー")
        || status.starts_with("出力失敗")
        || status.starts_with("PSD出力失敗")
    {
        StatusStyle {
            background: COLORS.error_bg,
            stroke: Stroke::new(METRICS.border_width, COLORS.error_border),
            text: COLORS.error_text,
        }
    } else if status.contains("完了しました") || status.contains("出力しました") {
        StatusStyle {
            background: COLORS.success_bg,
            stroke: Stroke::new(METRICS.border_width, COLORS.success_border),
            text: COLORS.success_text,
        }
    } else {
        StatusStyle {
            background: visuals.extreme_bg_color,
            stroke: visuals.widgets.noninteractive.bg_stroke,
            text: visuals.text_color(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_use_shared_metrics_and_current_visuals() {
        let mut style = Style::default();
        style.visuals.window_fill = Color32::BLUE;
        style.visuals.widgets.noninteractive.bg_fill = Color32::GREEN;
        style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(2.0_f32, Color32::RED);

        let pane = pane_frame(&style);
        assert_eq!(pane.fill, Color32::BLUE);
        assert_eq!(pane.stroke, style.visuals.widgets.noninteractive.bg_stroke);
        assert_eq!(pane.rounding, egui::Rounding::same(METRICS.pane_rounding));
        assert_eq!(pane.inner_margin, egui::Margin::same(METRICS.pane_padding));

        let card = card_frame(&style);
        assert_eq!(card.fill, Color32::GREEN);
        assert_eq!(card.stroke, style.visuals.widgets.noninteractive.bg_stroke);
        assert_eq!(card.rounding, egui::Rounding::same(METRICS.card_rounding));
        assert_eq!(card.inner_margin, egui::Margin::same(METRICS.card_padding));
    }

    #[test]
    fn error_messages_use_the_error_palette() {
        let visuals = Visuals::light();
        for message in [
            "エラー: 入力なし",
            "出力失敗: test",
            "PSD出力失敗: test",
            "エラー: 出力しました",
        ] {
            let style = status_style(message, &visuals);
            assert_eq!(style.background, COLORS.error_bg);
            assert_eq!(
                style.stroke,
                Stroke::new(METRICS.border_width, COLORS.error_border)
            );
            assert_eq!(style.text, COLORS.error_text);
        }
    }

    #[test]
    fn success_messages_use_the_success_palette() {
        let visuals = Visuals::light();
        for message in ["変換が完了しました。", "PSDファイルを出力しました。"]
        {
            let style = status_style(message, &visuals);
            assert_eq!(style.background, COLORS.success_bg);
            assert_eq!(
                style.stroke,
                Stroke::new(METRICS.border_width, COLORS.success_border)
            );
            assert_eq!(style.text, COLORS.success_text);
        }
    }

    #[test]
    fn neutral_messages_use_current_visuals() {
        let mut visuals = Visuals::light();
        visuals.extreme_bg_color = Color32::BLUE;
        visuals.override_text_color = Some(Color32::RED);
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(2.0_f32, Color32::GREEN);
        for message in ["準備完了", "PSDファイルを書き出し中...", ""] {
            let style = status_style(message, &visuals);
            assert_eq!(style.background, visuals.extreme_bg_color);
            assert_eq!(style.stroke, visuals.widgets.noninteractive.bg_stroke);
            assert_eq!(style.text, visuals.text_color());
        }
    }
}
