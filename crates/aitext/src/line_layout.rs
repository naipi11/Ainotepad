use std::sync::Arc;

use egui::{Color32, FontId, Galley, Ui};

pub struct EditorLineLayout {
    pub galley: Arc<Galley>,
    pub start_offset: usize,
    pub end_offset: usize,
}

impl EditorLineLayout {
    pub fn x_for_offset(&self, offset: usize) -> f32 {
        let local = offset.clamp(self.start_offset, self.end_offset) - self.start_offset;
        self.galley
            .rows
            .first()
            .map(|row| row.x_offset(local))
            .unwrap_or(0.0)
    }

    pub fn offset_at_x(&self, x: f32) -> usize {
        let column = self
            .galley
            .rows
            .first()
            .map(|row| row.char_at(x.max(0.0)))
            .unwrap_or(0);
        (self.start_offset + column).min(self.end_offset)
    }

    pub fn height(&self) -> f32 {
        self.galley
            .rows
            .first()
            .map(|row| row.rect.height())
            .unwrap_or(0.0)
    }
}

pub fn build_line_layout(
    ui: &Ui,
    font: &FontId,
    text: &str,
    spans: &[(usize, usize, Color32)],
) -> EditorLineLayout {
    let chars: Vec<char> = text.chars().collect();
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = f32::INFINITY;
    job.break_on_newline = false;
    let default_format = egui::text::TextFormat::simple(font.clone(), Color32::WHITE);
    let mut cursor = 0;
    if spans.is_empty() {
        job.append(text, 0.0, default_format);
    } else {
        for &(raw_start, raw_end, color) in spans {
            let start = raw_start.clamp(cursor, chars.len());
            let end = raw_end.clamp(start, chars.len());
            if start > cursor {
                let gap: String = chars[cursor..start].iter().collect();
                job.append(&gap, 0.0, default_format.clone());
            }
            if end > start {
                let segment: String = chars[start..end].iter().collect();
                job.append(
                    &segment,
                    0.0,
                    egui::text::TextFormat::simple(font.clone(), color),
                );
                cursor = end;
            }
        }
        if cursor < chars.len() {
            let tail: String = chars[cursor..].iter().collect();
            job.append(&tail, 0.0, default_format);
        }
    }
    let galley = ui.fonts(|fonts| fonts.layout_job(job));
    EditorLineLayout {
        galley,
        start_offset: 0,
        end_offset: chars.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{pos2, vec2, CentralPanel, Context, FontId, Margin, RawInput, Rect};

    fn build_test_layout(text: &str) -> EditorLineLayout {
        let context = Context::default();
        let mut result = None;
        let mut input = RawInput::default();
        input.screen_rect = Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(500.0, 120.0)));
        let _ = context.run(input, |context| {
            CentralPanel::default()
                .frame(egui::Frame::NONE.inner_margin(Margin::ZERO))
                .show(context, |ui| {
                    result = Some(build_line_layout(
                        ui,
                        &FontId::proportional(16.0),
                        text,
                        &[],
                    ));
                });
        });
        result.expect("line layout should be built")
    }

    #[test]
    fn mixed_script_layout_uses_one_galley() {
        let layout = build_test_layout("你好###abccABCDA你好###");
        assert_eq!(layout.galley.job.text, "你好###abccABCDA你好###");
        assert!(layout.x_for_offset(6) > layout.x_for_offset(5));
        assert_eq!(layout.offset_at_x(layout.x_for_offset(5) + 0.1), 5);
    }
}
