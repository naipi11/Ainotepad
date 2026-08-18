use aitext_core::{highlight, Document, Offset};
use egui::{pos2, vec2, FontId, Pos2, Rect, Sense, Ui};

use crate::config::AppConfig;
use crate::theme::{colors, token_color};

pub fn char_index_at(doc: &Document, font: &FontId, ui: &Ui, origin: Pos2, pos: Pos2) -> Offset {
    let galley = ui.fonts(|f| f.layout_no_wrap("M".into(), font.clone(), egui::Color32::WHITE));
    let w = galley.size().x.max(1.0);
    let h = galley.size().y.max(1.0);
    let rel = pos - origin;
    let line = ((rel.y / h).floor() as isize).max(0) as usize;
    let col = ((rel.x / w).round() as isize).max(0) as usize;
    let line = line.min(doc.line_count().saturating_sub(1));
    let start = line_start(doc, line);
    let end = line_end(doc, line);
    (start + col).min(end)
}

pub fn paint_editor(ui: &mut Ui, doc: &mut Document, config: &AppConfig, ghost: Option<&str>) {
    let theme = colors(config.theme);
    let font = FontId::monospace(config.font_size);
    let sample = ui.fonts(|f| f.layout_no_wrap("M".into(), font.clone(), theme.text));
    let cw = sample.size().x.max(1.0);
    let ch = sample.size().y.max(1.0);
    let line_count = doc.line_count().max(1);
    let gutter_w = 48.0;
    let desired = vec2(ui.available_width(), ui.available_height());
    let (rect, response) = ui.allocate_exact_size(desired, Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, theme.background);

    let caret = doc.selection().caret;
    let current_line = if doc.len_chars() == 0 {
        0
    } else {
        line_of(doc, caret)
    };
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

    let line_rect = Rect::from_min_size(
        pos2(rect.left(), origin.y + current_line as f32 * ch),
        vec2(rect.width(), ch),
    );
    painter.rect_filled(line_rect, 0.0, theme.current_line);

    let tokens = highlight(&doc.text(), doc.language());
    let text = doc.text();
    let chars: Vec<char> = text.chars().collect();
    for line in 0..line_count {
        let y = origin.y + line as f32 * ch;
        painter.text(
            pos2(rect.left() + 8.0, y),
            egui::Align2::LEFT_TOP,
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
            let ch = chars.get(i).copied().unwrap_or(' ');
            if ch != '\n' && ch != '\r' {
                painter.text(
                    pos2(x, y),
                    egui::Align2::LEFT_TOP,
                    ch.to_string(),
                    font.clone(),
                    token_color(config.theme, kind),
                );
                x += cw;
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
            painter.rect_filled(
                Rect::from_min_size(pos2(origin.x + from as f32 * cw, y), vec2((to.saturating_sub(from)) as f32 * cw, ch)),
                0.0,
                theme.selection,
            );
        }
    }

    let (cl, cc) = offset_line_col(doc, caret);
    let caret_pos = pos2(origin.x + cc as f32 * cw, origin.y + cl as f32 * ch);
    painter.rect_filled(
        Rect::from_min_size(caret_pos, vec2(1.5, ch)),
        0.0,
        theme.text,
    );

    if let Some(ghost) = ghost {
        painter.text(
            caret_pos,
            egui::Align2::LEFT_TOP,
            ghost,
            font,
            egui::Color32::from_rgba_unmultiplied(
                config.ghost_color[0],
                config.ghost_color[1],
                config.ghost_color[2],
                config.ghost_color[3],
            ),
        );
    }
}

fn line_of(doc: &Document, _offset: Offset) -> usize {
    if doc.len_chars() == 0 {
        0
    } else {
        doc.line_column().0.saturating_sub(1)
    }
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
