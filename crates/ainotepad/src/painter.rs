use ainotepad_core::{highlight, Document, Offset, Token};
use egui::{pos2, vec2, Color32, FontId, Pos2, Rect, Sense, Ui};

use crate::config::AppConfig;
use crate::line_layout::{build_line_layout, EditorLineLayout};
use crate::theme::{colors_with_custom, token_color_with_custom};

const EDITOR_GUTTER_WIDTH: f32 = 34.0;
const EDITOR_TEXT_INSET: f32 = 4.0;
const LINE_NUMBER_INSET: f32 = 6.0;

pub struct EditorPaintOutput {
    pub response: egui::Response,
    pub caret_changed: bool,
}

fn neutral_divider_color(gutter: Color32) -> Color32 {
    let gray = ((gutter.r() as u16 + gutter.g() as u16 + gutter.b() as u16) / 3) as u8;
    Color32::from_rgba_unmultiplied(gray, gray, gray, 110)
}

fn editor_font(config: &AppConfig, document_text: &str) -> FontId {
    let selected_family = if crate::app::font_is_available(&config.font_family) {
        egui::FontFamily::Name(config.font_family.clone().into())
    } else {
        egui::FontFamily::Monospace
    };
    let has_non_ascii = document_text
        .chars()
        .any(|character| !character.is_ascii() && !character.is_whitespace());
    let family = if has_non_ascii && crate::app::font_is_available("YaHei") {
        egui::FontFamily::Name("YaHei".into())
    } else {
        selected_family
    };
    FontId::new(config.font_size, family)
}

pub fn char_index_at(
    doc: &Document,
    layouts: &[EditorLineLayout],
    origin: Pos2,
    line_height: f32,
    pos: Pos2,
) -> Offset {
    let rel = pos - origin;
    let line = ((rel.y / line_height.max(1.0)).floor() as isize).max(0) as usize;
    let line = line.min(doc.line_count().saturating_sub(1));
    layouts
        .get(line)
        .map(|layout| layout.offset_at_x(rel.x))
        .unwrap_or_else(|| line_start(doc, line))
}

pub fn paint_editor(
    ui: &mut Ui,
    doc: &mut Document,
    config: &AppConfig,
    ghost: Option<&str>,
    preedit: Option<&str>,
) -> EditorPaintOutput {
    let theme = colors_with_custom(config.theme, &config.custom_theme);
    let font = editor_font(config, &doc.text());
    let line_count = doc.line_count().max(1);
    let tokens = highlight(&doc.text(), doc.language());
    let layouts = build_editor_line_layouts(ui, &font, doc, &tokens, config);
    let line_height = layouts
        .iter()
        .map(EditorLineLayout::height)
        .fold(0.0, f32::max)
        .max(ui.fonts(|fonts| fonts.row_height(&font)))
        .max(1.0);
    let desired = vec2(ui.available_width(), ui.available_height());
    let (rect, response) = ui.allocate_exact_size(desired, Sense::click_and_drag());
    if !ui.ctx().memory(|m| m.focused().is_some()) || response.clicked() {
        response.request_focus();
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
    }

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, theme.paper);
    let rule_x = rect.left() + EDITOR_GUTTER_WIDTH;
    painter.line_segment(
        [pos2(rule_x, rect.top()), pos2(rule_x, rect.bottom())],
        egui::Stroke::new(1.0_f32, neutral_divider_color(theme.gutter)),
    );

    let caret = doc.selection().caret;
    let current_line = offset_line_col(doc, caret).0;
    let origin = pos2(rule_x + EDITOR_TEXT_INSET, rect.top() + 4.0);

    let mut caret_changed = false;
    if response.clicked() || response.dragged() {
        if let Some(pos) = response.interact_pointer_pos() {
            let offset = char_index_at(doc, &layouts, origin, line_height, pos);
            if response.clicked() && !ui.input(|i| i.modifiers.shift) {
                if doc.selection().caret != offset || !doc.selection().is_empty() {
                    caret_changed = true;
                }
                doc.set_caret(offset);
            } else {
                let mut selection = doc.selection();
                if selection.caret != offset {
                    caret_changed = true;
                }
                selection.caret = offset;
                doc.set_selection(selection);
            }
        }
    }

    let line_rect = Rect::from_min_size(
        pos2(rect.left(), origin.y + current_line as f32 * line_height),
        vec2(rect.width(), line_height),
    );
    painter.rect_filled(line_rect, 0.0, theme.current_line);

    for (line, layout) in layouts.iter().enumerate().take(line_count) {
        let y = origin.y + line as f32 * line_height;
        painter.text(
            pos2(rule_x - LINE_NUMBER_INSET, y),
            egui::Align2::RIGHT_TOP,
            format!("{}", line + 1),
            font.clone(),
            theme.gutter,
        );
        painter.galley(pos2(origin.x, y), layout.galley.clone(), theme.text);
    }

    if !doc.selection().is_empty() {
        let start = doc.selection().start();
        let end = doc.selection().end();
        let (start_line, start_column) = offset_line_col(doc, start);
        let (end_line, end_column) = offset_line_col(doc, end);
        for line in start_line..=end_line {
            let from = if line == start_line {
                line_start(doc, line) + start_column
            } else {
                line_start(doc, line)
            };
            let to = if line == end_line {
                line_start(doc, line) + end_column
            } else {
                line_end(doc, line)
            };
            if let Some(layout) = layouts.get(line) {
                let x0 = origin.x + layout.x_for_offset(from);
                let x1 = origin.x + layout.x_for_offset(to);
                painter.rect_filled(
                    Rect::from_min_size(
                        pos2(x0, origin.y + line as f32 * line_height),
                        vec2((x1 - x0).max(1.0), line_height),
                    ),
                    0.0,
                    theme.selection,
                );
            }
        }
    }

    let (current_line, _) = offset_line_col(doc, caret);
    let mut caret_x = origin.x
        + layouts
            .get(current_line)
            .map(|layout| layout.x_for_offset(caret))
            .unwrap_or(0.0);
    let caret_y = origin.y + current_line as f32 * line_height;

    if let Some(preedit) = preedit.filter(|text| !text.is_empty()) {
        let preedit_chars = preedit.chars().count();
        let preedit_layout =
            build_line_layout(ui, &font, preedit, &[(0, preedit_chars, theme.text)]);
        painter.galley(
            pos2(caret_x, caret_y),
            preedit_layout.galley.clone(),
            theme.text,
        );
        caret_x += preedit_layout.x_for_offset(preedit_chars);
    }

    let caret_pos = pos2(caret_x, caret_y);
    painter.rect_filled(
        Rect::from_min_size(caret_pos, vec2(1.5, line_height)),
        0.0,
        theme.text,
    );

    if let Some(ghost) = ghost.filter(|text| !text.is_empty()) {
        let ghost_chars = ghost.chars().count();
        let ghost_layout = build_line_layout(ui, &font, ghost, &[(0, ghost_chars, theme.ghost)]);
        painter.galley(caret_pos, ghost_layout.galley, theme.ghost);
    }

    let caret_rect = Rect::from_min_size(caret_pos, vec2(2.0, line_height));
    ui.ctx().output_mut(|output| {
        output.ime = Some(egui::output::IMEOutput {
            rect: caret_rect.expand2(vec2(12.0, 6.0)),
            cursor_rect: caret_rect,
        });
        output.mutable_text_under_cursor = true;
    });
    EditorPaintOutput {
        response,
        caret_changed,
    }
}

fn build_editor_line_layouts(
    ui: &Ui,
    font: &FontId,
    doc: &Document,
    tokens: &[Token],
    config: &AppConfig,
) -> Vec<EditorLineLayout> {
    let chars: Vec<char> = doc.text().chars().collect();
    (0..doc.line_count().max(1))
        .map(|line| {
            let start = line_start(doc, line);
            let end = line_end(doc, line);
            let text: String = chars[start..end].iter().collect();
            let spans: Vec<(usize, usize, Color32)> = tokens
                .iter()
                .filter_map(|token| {
                    let overlap_start = token.start.max(start);
                    let overlap_end = token.end.min(end);
                    (overlap_start < overlap_end).then(|| {
                        (
                            overlap_start - start,
                            overlap_end - start,
                            token_color_with_custom(config.theme, &config.custom_theme, token.kind),
                        )
                    })
                })
                .collect();
            let mut layout = build_line_layout(ui, font, &text, &spans);
            layout.start_offset = start;
            layout.end_offset = end;
            layout
        })
        .collect()
}

fn line_start(doc: &Document, line: usize) -> Offset {
    let mut count = 0;
    let mut offset = 0;
    for ch in doc.text().chars() {
        if count == line {
            return offset;
        }
        offset += 1;
        if ch == '\n' {
            count += 1;
        }
    }
    offset
}

fn line_end(doc: &Document, line: usize) -> Offset {
    let start = line_start(doc, line);
    let mut offset = start;
    for ch in doc.text().chars().skip(start) {
        if ch == '\n' {
            return offset;
        }
        offset += 1;
    }
    offset
}

fn offset_line_col(doc: &Document, offset: Offset) -> (usize, usize) {
    let mut line = 0;
    let mut column = 0;
    for (index, ch) in doc.text().chars().enumerate() {
        if index == offset {
            return (line, column);
        }
        if ch == '\n' {
            line += 1;
            column = 0;
        } else {
            column += 1;
        }
    }
    (line, column)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_gutter_is_compact_and_uses_a_neutral_divider() {
        let context = egui::Context::default();
        let mut input = egui::RawInput::default();
        input.screen_rect = Some(Rect::from_min_size(Pos2::ZERO, vec2(300.0, 160.0)));
        let mut document = Document::new();
        let mut config = AppConfig::default();
        config.font_family = "__aitext_test_missing_font__".into();
        let mut editor_left = 0.0;

        let output = context.run(input, |context| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.inner_margin(egui::Margin::ZERO))
                .show(context, |ui| {
                    editor_left = ui.available_rect_before_wrap().left();
                    paint_editor(ui, &mut document, &config, None, None);
                });
        });

        let (divider_x, divider_color) = output
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::LineSegment { points, stroke }
                    if (points[0].x - points[1].x).abs() < f32::EPSILON
                        && (points[1].y - points[0].y).abs() > 100.0 =>
                {
                    Some((points[0].x, stroke.color))
                }
                _ => None,
            })
            .expect("editor divider should be painted");
        let caret_x = output
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(shape)
                    if (shape.rect.width() - 1.5).abs() < 0.01 && shape.rect.height() > 5.0 =>
                {
                    Some(shape.rect.left())
                }
                _ => None,
            })
            .expect("initial caret should be painted");

        assert_eq!(divider_x - editor_left, 34.0);
        assert_eq!(caret_x - divider_x, 4.0);
        assert_eq!(divider_color.r(), divider_color.g());
        assert_eq!(divider_color.g(), divider_color.b());
    }

    #[test]
    fn mixed_script_text_is_painted_as_one_line_galley() {
        let context = egui::Context::default();
        crate::app::install_fonts(&context);
        let mut input = egui::RawInput::default();
        input.screen_rect = Some(Rect::from_min_size(Pos2::ZERO, vec2(600.0, 180.0)));
        let mut document = Document::from_text("你好###abccABCDA你好###");
        document.set_caret(5);
        let mut config = AppConfig::default();
        config.font_family = "__aitext_test_missing_font__".into();

        let output = context.run(input, |context| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.inner_margin(egui::Margin::ZERO))
                .show(context, |ui| {
                    paint_editor(ui, &mut document, &config, Some("建议"), None);
                });
        });

        let line_galley = output
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(shape) if shape.galley.job.text == "你好###abccABCDA你好###" => {
                    Some(shape)
                }
                _ => None,
            });
        assert!(
            line_galley.is_some(),
            "mixed line should use one shaped galley"
        );

        let caret_x = output
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(shape)
                    if (shape.rect.width() - 1.5).abs() < 0.01 && shape.rect.height() > 5.0 =>
                {
                    Some(shape.rect.left())
                }
                _ => None,
            });
        let ghost_x = output
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(shape) if shape.galley.job.text == "建议" => Some(shape.pos.x),
                _ => None,
            });
        assert!((ghost_x.unwrap() - caret_x.unwrap()).abs() < 0.01);
    }

    #[test]
    fn mixed_script_document_uses_cjk_primary_font() {
        let mut config = AppConfig::default();
        config.font_family = "Consolas".into();
        let mixed = editor_font(&config, "你好ABC");
        let ascii = editor_font(&config, "fn main() {}");
        assert_eq!(mixed.family, egui::FontFamily::Name("YaHei".into()));
        assert_eq!(ascii.family, egui::FontFamily::Name("Consolas".into()));
    }

    #[test]
    fn editor_hover_uses_a_text_cursor() {
        let context = egui::Context::default();
        let mut input = egui::RawInput::default();
        input.screen_rect = Some(Rect::from_min_size(Pos2::ZERO, vec2(300.0, 160.0)));
        let mut document = Document::new();
        let mut config = AppConfig::default();
        config.font_family = "__aitext_test_missing_font__".into();

        let _ = context.run(input, |context| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.inner_margin(egui::Margin::ZERO))
                .show(context, |ui| {
                    paint_editor(ui, &mut document, &config, None, None);
                });
        });

        let mut hovered_input = egui::RawInput::default();
        hovered_input.screen_rect = Some(Rect::from_min_size(Pos2::ZERO, vec2(300.0, 160.0)));
        hovered_input
            .events
            .push(egui::Event::PointerMoved(pos2(80.0, 20.0)));
        let output = context.run(hovered_input, |context| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.inner_margin(egui::Margin::ZERO))
                .show(context, |ui| {
                    paint_editor(ui, &mut document, &config, None, None);
                });
        });

        assert_eq!(output.platform_output.cursor_icon, egui::CursorIcon::Text);
    }
}
