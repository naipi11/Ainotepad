use aitext_core::{highlight, Document, Offset};
use egui::{pos2, vec2, FontId, Galley, Pos2, Rect, Sense, Ui};

use crate::config::AppConfig;
use crate::theme::{colors_with_custom, token_color_with_custom};

const EDITOR_GUTTER_WIDTH: f32 = 34.0;
const EDITOR_TEXT_INSET: f32 = 4.0;
const LINE_NUMBER_INSET: f32 = 6.0;

fn neutral_divider_color(gutter: egui::Color32) -> egui::Color32 {
    let gray = ((gutter.r() as u16 + gutter.g() as u16 + gutter.b() as u16) / 3) as u8;
    egui::Color32::from_rgba_unmultiplied(gray, gray, gray, 110)
}

fn layout_char(ui: &Ui, font: &FontId, ch: char, color: egui::Color32) -> std::sync::Arc<Galley> {
    ui.fonts(|f| f.layout_no_wrap(ch.to_string(), font.clone(), color))
}

fn char_advance(ui: &Ui, font: &FontId, ch: char) -> f32 {
    if ch == '\t' {
        return layout_char(ui, font, ' ', egui::Color32::WHITE).size().x * 4.0;
    }
    let galley = layout_char(ui, font, ch, egui::Color32::WHITE);
    let w = galley.size().x;
    if w > 0.5 {
        w
    } else if (ch as u32) > 0x7F {
        layout_char(ui, font, 'M', egui::Color32::WHITE).size().x * 2.0
    } else {
        layout_char(ui, font, 'M', egui::Color32::WHITE)
            .size()
            .x
            .max(1.0)
    }
}

fn line_x_for_offset(ui: &Ui, font: &FontId, doc: &Document, line: usize, offset: Offset) -> f32 {
    let start = line_start(doc, line);
    let target = offset.min(line_end(doc, line));
    let mut x = 0.0;
    for (i, ch) in doc.text().chars().enumerate().skip(start) {
        if i >= target || ch == '\n' || ch == '\r' {
            break;
        }
        x += char_advance(ui, font, ch);
    }
    x
}

pub fn char_index_at(doc: &Document, font: &FontId, ui: &Ui, origin: Pos2, pos: Pos2) -> Offset {
    let sample = layout_char(ui, font, 'M', egui::Color32::WHITE);
    let h = sample.size().y.max(1.0);
    let rel = pos - origin;
    let line = ((rel.y / h).floor() as isize).max(0) as usize;
    let line = line.min(doc.line_count().saturating_sub(1));
    let start = line_start(doc, line);
    let end = line_end(doc, line);
    let mut x = 0.0;
    let mut chosen = start;
    for (i, ch) in doc.text().chars().enumerate().skip(start) {
        if i >= end || ch == '\n' || ch == '\r' {
            break;
        }
        let w = char_advance(ui, font, ch);
        if rel.x < x + w / 2.0 {
            return i;
        }
        x += w;
        chosen = i + 1;
    }
    chosen.min(end)
}

pub fn paint_editor(
    ui: &mut Ui,
    doc: &mut Document,
    config: &AppConfig,
    ghost: Option<&str>,
    preedit: Option<&str>,
) -> bool {
    let theme = colors_with_custom(config.theme, &config.custom_theme);
    let family = if crate::app::font_is_available(&config.font_family) {
        egui::FontFamily::Name(config.font_family.clone().into())
    } else {
        egui::FontFamily::Monospace
    };
    let font = FontId::new(config.font_size, family);
    let sample = layout_char(ui, &font, 'M', theme.text);
    let ch = sample.size().y.max(1.0);
    let line_count = doc.line_count().max(1);
    let gutter_w = EDITOR_GUTTER_WIDTH;
    let desired = vec2(ui.available_width(), ui.available_height());
    let (rect, response) = ui.allocate_exact_size(desired, Sense::click_and_drag());
    if !ui.ctx().memory(|m| m.focused().is_some()) || response.clicked() {
        response.request_focus();
    }
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, theme.paper);
    let rule_x = rect.left() + gutter_w;
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
            let offset = char_index_at(doc, &font, ui, origin, pos);
            if response.clicked() && !ui.input(|i| i.modifiers.shift) {
                if doc.selection().caret != offset || !doc.selection().is_empty() {
                    caret_changed = true;
                }
                doc.set_caret(offset);
            } else {
                let mut sel = doc.selection();
                if sel.caret != offset {
                    caret_changed = true;
                }
                sel.caret = offset;
                doc.set_selection(sel);
            }
        }
    }

    let line_rect = Rect::from_min_size(
        pos2(rect.left(), origin.y + current_line as f32 * ch),
        vec2(rect.width(), ch),
    );
    painter.rect_filled(line_rect, 0.0, theme.current_line);

    let tokens = highlight(&doc.text(), doc.language());
    let chars: Vec<char> = doc.text().chars().collect();
    for line in 0..line_count {
        let y = origin.y + line as f32 * ch;
        painter.text(
            pos2(rule_x - LINE_NUMBER_INSET, y),
            egui::Align2::RIGHT_TOP,
            format!("{}", line + 1),
            font.clone(),
            theme.gutter,
        );
        let start = line_start(doc, line);
        let end = line_end(doc, line);
        let mut x = origin.x;
        let mut i = start;
        while i < end {
            let kind = tokens
                .iter()
                .find(|t| t.start <= i && i < t.end)
                .map(|t| t.kind)
                .unwrap_or(aitext_core::TokenKind::Text);
            let glyph = chars.get(i).copied().unwrap_or(' ');
            if glyph != '\n' && glyph != '\r' {
                let galley = layout_char(
                    ui,
                    &font,
                    glyph,
                    token_color_with_custom(config.theme, &config.custom_theme, kind),
                );
                painter.galley(
                    pos2(x, y),
                    galley.clone(),
                    token_color_with_custom(config.theme, &config.custom_theme, kind),
                );
                x += char_advance(ui, &font, glyph);
            }
            i += 1;
        }
    }

    if !doc.selection().is_empty() {
        let start = doc.selection().start();
        let end = doc.selection().end();
        let (sl, sc) = offset_line_col(doc, start);
        let (el, ec) = offset_line_col(doc, end);
        for line in sl..=el {
            let from = if line == sl { sc } else { 0 };
            let to = if line == el { ec } else { line_len(doc, line) };
            let y = origin.y + line as f32 * ch;
            let x0 =
                origin.x + line_x_for_offset(ui, &font, doc, line, line_start(doc, line) + from);
            let x1 = origin.x + line_x_for_offset(ui, &font, doc, line, line_start(doc, line) + to);
            painter.rect_filled(
                Rect::from_min_size(pos2(x0, y), vec2((x1 - x0).max(1.0), ch)),
                0.0,
                theme.selection,
            );
        }
    }

    let (cl, _) = offset_line_col(doc, caret);
    let mut caret_x = origin.x + line_x_for_offset(ui, &font, doc, cl, caret);
    if let Some(preedit) = preedit {
        for glyph in preedit.chars() {
            if glyph == '\n' || glyph == '\r' {
                continue;
            }
            let galley = layout_char(ui, &font, glyph, theme.text);
            painter.galley(
                pos2(caret_x, origin.y + cl as f32 * ch),
                galley.clone(),
                theme.text,
            );
            caret_x += char_advance(ui, &font, glyph);
        }
    }
    let caret_pos = pos2(caret_x, origin.y + cl as f32 * ch);
    painter.rect_filled(
        Rect::from_min_size(caret_pos, vec2(1.5, ch)),
        0.0,
        theme.text,
    );

    if let Some(ghost) = ghost {
        painter.text(caret_pos, egui::Align2::LEFT_TOP, ghost, font, theme.ghost);
    }

    let caret_rect = Rect::from_min_size(caret_pos, vec2(2.0, ch));
    ui.ctx().output_mut(|o| {
        o.ime = Some(egui::output::IMEOutput {
            rect: caret_rect.expand2(vec2(12.0, 6.0)),
            cursor_rect: caret_rect,
        });
        o.mutable_text_under_cursor = true;
    });
    caret_changed
}

fn line_start(doc: &Document, line: usize) -> Offset {
    let mut count = 0;
    let mut off = 0;
    for ch in doc.text().chars() {
        if count == line {
            return off;
        }
        off += 1;
        if ch == '\n' {
            count += 1;
        }
    }
    off
}

fn line_end(doc: &Document, line: usize) -> Offset {
    let start = line_start(doc, line);
    let mut off = start;
    for ch in doc.text().chars().skip(start) {
        if ch == '\n' {
            return off;
        }
        off += 1;
    }
    off
}

fn line_len(doc: &Document, line: usize) -> usize {
    line_end(doc, line).saturating_sub(line_start(doc, line))
}

fn offset_line_col(doc: &Document, offset: Offset) -> (usize, usize) {
    let mut line = 0;
    let mut col = 0;
    for (i, ch) in doc.text().chars().enumerate() {
        if i == offset {
            return (line, col);
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    (line, col)
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
}
