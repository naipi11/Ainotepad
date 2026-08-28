use crate::document::Document;
use crate::selection::Selection;
use crate::undo::Edit;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndentSettings {
    pub use_tabs: bool,
    pub width: usize,
}

impl Default for IndentSettings {
    fn default() -> Self {
        Self {
            use_tabs: false,
            width: 4,
        }
    }
}

fn unit(settings: IndentSettings) -> String {
    if settings.use_tabs {
        "\t".to_string()
    } else {
        " ".repeat(settings.width.max(1))
    }
}

impl Document {
    pub fn indent(&mut self, settings: IndentSettings) {
        if self.is_readonly() {
            return;
        }
        let unit = unit(settings);
        if self.selection().is_empty() || !self.selected_text().contains('\n') {
            self.insert(&unit);
            return;
        }
        self.transform_touched_lines(|line| format!("{unit}{line}"));
    }

    pub fn unindent(&mut self, settings: IndentSettings) {
        if self.is_readonly() {
            return;
        }
        let unit = unit(settings);
        self.transform_touched_lines(|line| {
            if let Some(rest) = line.strip_prefix(&unit) {
                rest.to_string()
            } else if !settings.use_tabs {
                let mut i = 0;
                let chars: Vec<char> = line.chars().collect();
                while i < chars.len() && i < settings.width && chars[i] == ' ' {
                    i += 1;
                }
                chars[i..].iter().collect()
            } else {
                line.to_string()
            }
        });
    }

    fn transform_touched_lines(&mut self, mut f: impl FnMut(&str) -> String) {
        let sel = self.selection();
        let start = sel.start();
        let end = sel.end();
        let first_line = self.line_of(start);
        let last_line = if end > start {
            let last = self.line_of(end);
            if self.start_of_line(last) == end && last > first_line {
                last - 1
            } else {
                last
            }
        } else {
            first_line
        };
        let before = sel;
        let old = self.text();
        let chars: Vec<char> = old.chars().collect();
        let mut out = String::new();
        let mut last = 0;
        for line in first_line..=last_line {
            let ls = self.start_of_line(line);
            let le = if line + 1 < self.line_count() {
                self.start_of_line(line + 1)
            } else {
                self.len_chars()
            };
            out.extend(chars[last..ls].iter().copied());
            let raw: String = chars[ls..le].iter().collect();
            let (content, nl) = if let Some(stripped) = raw.strip_suffix("\r\n") {
                (stripped.to_string(), "\r\n")
            } else if let Some(stripped) = raw.strip_suffix('\n') {
                (stripped.to_string(), "\n")
            } else {
                (raw, "")
            };
            out.push_str(&f(&content));
            out.push_str(nl);
            last = le;
        }
        out.extend(chars[last..].iter().copied());
        self.rope_replace_all(&out);
        let after = Selection {
            anchor: 0,
            caret: self.len_chars(),
        };
        self.set_selection(after);
        self.push_public_edit(Edit {
            delete_start: 0,
            deleted: old,
            insert_start: 0,
            inserted: out,
            before,
            after,
            coalesce_inserts: false,
        });
        self.mark_dirty();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::selection::Selection;

    #[test]
    fn indent_inserts_spaces_at_caret() {
        let mut doc = Document::from_text("ab");
        doc.set_caret(0);
        doc.indent(IndentSettings {
            use_tabs: false,
            width: 4,
        });
        assert_eq!(doc.text(), "    ab");
        assert_eq!(doc.selection().caret, 4);
    }

    #[test]
    fn indent_and_unindent_block() {
        let mut doc = Document::from_text("a\nb");
        doc.set_selection(Selection {
            anchor: 0,
            caret: 3,
        });
        let s = IndentSettings {
            use_tabs: false,
            width: 2,
        };
        doc.indent(s);
        assert_eq!(doc.text(), "  a\n  b");
        doc.unindent(s);
        assert_eq!(doc.text(), "a\nb");
    }
}
