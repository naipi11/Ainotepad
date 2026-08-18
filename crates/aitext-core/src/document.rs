use ropey::Rope;

use crate::selection::{Offset, Selection};

#[derive(Clone, Debug)]
pub struct Document {
    rope: Rope,
    selection: Selection,
    dirty: bool,
    readonly: bool,
}

impl Document {
    pub fn new() -> Self {
        Self::from_text("")
    }

    pub fn from_text(text: impl Into<String>) -> Self {
        let text = text.into();
        Self {
            rope: Rope::from_str(&text),
            selection: Selection::default(),
            dirty: false,
            readonly: false,
        }
    }

    pub fn text(&self) -> String {
        self.rope.to_string()
    }

    pub fn len_chars(&self) -> Offset {
        self.rope.len_chars()
    }

    pub fn is_empty(&self) -> bool {
        self.rope.len_chars() == 0
    }

    pub fn selection(&self) -> Selection {
        self.selection
    }

    pub fn set_caret(&mut self, caret: Offset) {
        let caret = self.clamp(caret);
        self.selection = Selection {
            anchor: caret,
            caret,
        };
    }

    pub fn set_selection(&mut self, selection: Selection) {
        self.selection = Selection {
            anchor: self.clamp(selection.anchor),
            caret: self.clamp(selection.caret),
        };
    }

    pub fn insert(&mut self, text: &str) {
        if self.readonly {
            return;
        }
        if !self.selection.is_empty() {
            self.delete_range(self.selection.start(), self.selection.end());
        }
        let at = self.selection.caret;
        self.rope.insert(at, text);
        let caret = at + text.chars().count();
        self.selection = Selection {
            anchor: caret,
            caret,
        };
        self.dirty = true;
    }

    pub fn delete_backward(&mut self) {
        if self.readonly {
            return;
        }
        if !self.selection.is_empty() {
            self.delete_range(self.selection.start(), self.selection.end());
            self.dirty = true;
            return;
        }
        if self.selection.caret == 0 {
            return;
        }
        let end = self.selection.caret;
        self.delete_range(end - 1, end);
        self.dirty = true;
    }

    pub fn delete_forward(&mut self) {
        if self.readonly {
            return;
        }
        if !self.selection.is_empty() {
            self.delete_range(self.selection.start(), self.selection.end());
            self.dirty = true;
            return;
        }
        if self.selection.caret >= self.len_chars() {
            return;
        }
        let start = self.selection.caret;
        self.delete_range(start, start + 1);
        self.dirty = true;
    }

    pub fn replace_selection(&mut self, text: &str) {
        self.insert(text);
    }

    pub fn selected_text(&self) -> String {
        if self.selection.is_empty() {
            return String::new();
        }
        self.rope
            .slice(self.selection.start()..self.selection.end())
            .to_string()
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    pub fn is_readonly(&self) -> bool {
        self.readonly
    }

    pub fn set_readonly(&mut self, readonly: bool) {
        self.readonly = readonly;
    }

    fn clamp(&self, offset: Offset) -> Offset {
        offset.min(self.len_chars())
    }

    fn delete_range(&mut self, start: Offset, end: Offset) {
        if start == end {
            return;
        }
        self.rope.remove(start..end);
        self.selection = Selection {
            anchor: start,
            caret: start,
        };
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_at_caret_updates_text_and_caret() {
        let mut doc = Document::from_text("ab");
        doc.set_caret(1);
        doc.insert("X");
        assert_eq!(doc.text(), "aXb");
        assert_eq!(
            doc.selection(),
            Selection {
                anchor: 2,
                caret: 2
            }
        );
        assert!(doc.is_dirty());
    }

    #[test]
    fn insert_replaces_selection() {
        let mut doc = Document::from_text("hello");
        doc.set_selection(Selection {
            anchor: 1,
            caret: 4,
        });
        doc.insert("i");
        assert_eq!(doc.text(), "hio");
        assert_eq!(
            doc.selection(),
            Selection {
                anchor: 2,
                caret: 2
            }
        );
    }

    #[test]
    fn delete_backward_removes_previous_char_or_selection() {
        let mut doc = Document::from_text("ab");
        doc.set_caret(1);
        doc.delete_backward();
        assert_eq!(doc.text(), "b");
        doc.set_selection(Selection {
            anchor: 0,
            caret: 1,
        });
        doc.delete_backward();
        assert_eq!(doc.text(), "");
    }

    #[test]
    fn delete_forward_removes_next_char() {
        let mut doc = Document::from_text("ab");
        doc.set_caret(0);
        doc.delete_forward();
        assert_eq!(doc.text(), "b");
    }

    #[test]
    fn readonly_rejects_edits() {
        let mut doc = Document::from_text("keep");
        doc.mark_clean();
        doc.set_readonly(true);
        doc.insert("x");
        doc.delete_backward();
        assert_eq!(doc.text(), "keep");
        assert!(!doc.is_dirty());
    }

    #[test]
    fn chinese_chars_count_as_one_offset_each() {
        let mut doc = Document::from_text("你好");
        assert_eq!(doc.len_chars(), 2);
        doc.set_caret(1);
        doc.insert("啊");
        assert_eq!(doc.text(), "你啊好");
        assert_eq!(doc.len_chars(), 3);
    }
}
