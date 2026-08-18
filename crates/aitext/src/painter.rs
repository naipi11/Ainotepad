use aitext_core::{highlight, Document, Offset};
use egui::{pos2, vec2, FontId, Galley, Pos2, Rect, Sense, Ui};

use crate::config::AppConfig;
use crate::theme::{colors, token_color};

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
        layout_char(ui, font, 'M', egui::Color32::WHITE).size().x.max(1.0)
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

pub fn paint_editor(ui: &mut Ui, doc: &mut Document, config: &AppConfig, ghost: Option<&str>) {
    let theme = colors(config.theme);
    let font = FontId::monospace(config.font_size);
    let sample = layout_char(ui, &font, 'M', theme.text);
    let ch = sample.size().y.max(1.0);
    let line_count = doc.line_count().max(1);
    let gutter_w = 48.0;
    let desired = vec2(ui.available_width(), ui.available_height());
    let (rect, response) = ui.allocate_exact_size(desired, Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, theme.background);

    let caret = doc.selection().caret;
    let current_line = offset_line_col(doc, caret).0;
    let origin = pos2(rect.left() + gutter_w + 8.0, rect.top() + 4.0);

    if response.clicked() || response.dragged() {
        if let Some(pos) = response.interact_pointer_pos() {
            let offset = char_index_at(doc, &font, ui, origin, pos);
            if response.clicked() && !ui.input(|i| i.modifiers.shift) {
                doc.set_caret(offset);
            } else {
                let mut sel = doc.selection();
                sel.caret = offset;
                doc.set_selection(sel);
            }
        }
    }

    let line_rect = Rect::from_min_size(pos2(rect.left(), origin.y + current_line as f32 * ch), vec2(rect.width(), ch));
    painter.rect_filled(line_rect, 0.0, theme.current_line);

    let tokens = highlight(&doc.text(), doc.language());
    let chars: Vec<char> = doc.text().chars().collect();
    for line in 0..line_count {
        let y = origin.y + line as f32 * ch;
        painter.text(pos2(rect.left() + 8.0, y), egui::Align2::LEFT_TOP, format!("{}", line + 1), font.clone(), theme.gutter);
        let start = line_start(doc, line);
        let end = line_end(doc, line);
        let mut x = origin.x;
        let mut i = start;
        while i < end {
            let kind = tokens.iter().find(|t| t.start <= i && i < t.end).map(|t| t.kind).unwrap_or(aitext_core::TokenKind::Text);
            let glyph = chars.get(i).copied().unwrap_or(' ');
            if glyph != '\n' && glyph != '\r' {
                let galley = layout_char(ui, &font, glyph, token_color(config.theme, kind));
                painter.galley(pos2(x, y), galley.clone(), token_color(config.theme, kind));
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
            let x0 = origin.x + line_x_for_offset(ui, &font, doc, line, line_start(doc, line) + from);
            let x1 = origin.x + line_x_for_offset(ui, &font, doc, line, line_start(doc, line) + to);
            painter.rect_filled(Rect::from_min_size(pos2(x0, y), vec2((x1 - x0).max(1.0), ch)), 0.0, theme.selection);
        }
    }

    let (cl, _) = offset_line_col(doc, caret);
    let caret_x = origin.x + line_x_for_offset(ui, &font, doc, cl, caret);
    let caret_pos = pos2(caret_x, origin.y + cl as f32 * ch);
    painter.rect_filled(Rect::from_min_size(caret_pos, vec2(1.5, ch)), 0.0, theme.text);

    if let Some(ghost) = ghost {
        painter.text(caret_pos, egui::Align2::LEFT_TOP, ghost, font, egui::Color32::from_rgba_unmultiplied(config.ghost_color[0], config.ghost_color[1], config.ghost_color[2], config.ghost_color[3]));
    }

    let caret_rect = Rect::from_min_size(caret_pos, vec2(2.0, ch));
    ui.ctx().output_mut(|o| {
        o.ime = Some(egui::output::IMEOutput { rect: caret_rect.expand2(vec2(12.0, 6.0)), cursor_rect: caret_rect });
        o.mutable_text_under_cursor = true;
    });
}

fn line_start(doc: &Document, line: usize) -> Offset {
    let mut count = 0;
    let mut off = 0;
    for ch in doc.text().chars() {
        if count == line { return off; }
        off += 1;
        if ch == '\n' { count += 1; }
    }
    off
}

fn line_end(doc: &Document, line: usize) -> Offset {
    let start = line_start(doc, line);
    let mut off = start;
    for ch in doc.text().chars().skip(start) {
        if ch == '\n' { return off; }
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
        if i == offset { return (line, col); }
        if ch == '\n' { line += 1; col = 0; } else { col += 1; }
    }
    (line, col)
}
