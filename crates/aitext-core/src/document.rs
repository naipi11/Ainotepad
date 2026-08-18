use ropey::Rope;

use crate::selection::{Offset, Selection};
use crate::motion::{Motion, PAGE_LINES};
use crate::undo::Edit;

#[derive(Clone, Debug)]
pub struct Document {
    rope: Rope,
    selection: Selection,
    dirty: bool,
    readonly: bool,
    undo_stack: Vec<Edit>,
    redo_stack: Vec<Edit>,
    preferred_column: Option<usize>,
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
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            preferred_column: None,
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
        self.preferred_column = None;
    }

    pub fn set_selection(&mut self, selection: Selection) {
        self.selection = Selection {
            anchor: self.clamp(selection.anchor),
            caret: self.clamp(selection.caret),
        };
        self.preferred_column = None;
    }

    pub fn insert(&mut self, text: &str) {
        if self.readonly {
            return;
        }
        let before = self.selection;
        let deleted = if self.selection.is_empty() {
            String::new()
        } else {
            self.selected_text()
        };
        let delete_start = self.selection.start();
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
        let coalesce_inserts = deleted.is_empty() && text.chars().count() == 1;
        let edit = Edit {
            delete_start,
            deleted,
            insert_start: at,
            inserted: text.to_string(),
            before,
            after: self.selection,
            coalesce_inserts,
        };
        self.push_edit(edit);
        self.dirty = true;
    }

    pub fn delete_backward(&mut self) {
        if self.readonly {
            return;
        }
        let before = self.selection;
        if !self.selection.is_empty() {
            let deleted = self.selected_text();
            let start = self.selection.start();
            self.delete_range(self.selection.start(), self.selection.end());
            self.push_edit(Edit {
                delete_start: start,
                deleted,
                insert_start: start,
                inserted: String::new(),
                before,
                after: self.selection,
                coalesce_inserts: false,
            });
            self.dirty = true;
            return;
        }
        if self.selection.caret == 0 {
            return;
        }
        let end = self.selection.caret;
        let deleted = self.rope.slice((end - 1)..end).to_string();
        self.delete_range(end - 1, end);
        self.push_edit(Edit {
            delete_start: end - 1,
            deleted,
            insert_start: end - 1,
            inserted: String::new(),
            before,
            after: self.selection,
            coalesce_inserts: false,
        });
        self.dirty = true;
    }

    pub fn delete_forward(&mut self) {
        if self.readonly {
            return;
        }
        let before = self.selection;
        if !self.selection.is_empty() {
            let deleted = self.selected_text();
            let start = self.selection.start();
            self.delete_range(self.selection.start(), self.selection.end());
            self.push_edit(Edit {
                delete_start: start,
                deleted,
                insert_start: start,
                inserted: String::new(),
                before,
                after: self.selection,
                coalesce_inserts: false,
            });
            self.dirty = true;
            return;
        }
        if self.selection.caret >= self.len_chars() {
            return;
        }
        let start = self.selection.caret;
        let deleted = self.rope.slice(start..(start + 1)).to_string();
        self.delete_range(start, start + 1);
        self.push_edit(Edit {
            delete_start: start,
            deleted,
            insert_start: start,
            inserted: String::new(),
            before,
            after: self.selection,
            coalesce_inserts: false,
        });
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

    pub fn undo(&mut self) -> bool {
        let Some(edit) = self.undo_stack.pop() else {
            return false;
        };
        self.apply_inverse(&edit);
        self.redo_stack.push(edit);
        self.dirty = true;
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(edit) = self.redo_stack.pop() else {
            return false;
        };
        self.apply_forward(&edit);
        self.undo_stack.push(edit);
        self.dirty = true;
        true
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn move_caret(&mut self, motion: Motion, extend: bool) {
        let next = self.offset_after(motion);
        if extend {
            self.selection.caret = next;
        } else {
            self.selection = Selection {
                anchor: next,
                caret: next,
            };
        }
        match motion {
            Motion::Up | Motion::Down | Motion::PageUp | Motion::PageDown => {}
            _ => self.preferred_column = None,
        }
    }

    pub fn line_column(&self) -> (usize, usize) {
        if self.len_chars() == 0 {
            return (1, 1);
        }
        let caret = self.selection.caret.min(self.len_chars());
        let line = self.rope.char_to_line(caret.min(self.rope.len_chars()));
        let line_start = self.rope.line_to_char(line);
        (line + 1, caret - line_start + 1)
    }

    pub fn line_count(&self) -> usize {
        self.rope.len_lines()
    }

    fn clamp(&self, offset: Offset) -> Offset {
        offset.min(self.len_chars())
    }

    fn offset_after(&mut self, motion: Motion) -> Offset {
        let caret = self.selection.caret.min(self.len_chars());
        match motion {
            Motion::Left => caret.saturating_sub(1),
            Motion::Right => (caret + 1).min(self.len_chars()),
            Motion::Home => self.line_start(caret),
            Motion::End => self.line_end(caret),
            Motion::DocumentHome => 0,
            Motion::DocumentEnd => self.len_chars(),
            Motion::Up => self.vertical(caret, -1),
            Motion::Down => self.vertical(caret, 1),
            Motion::PageUp => self.vertical(caret, -(PAGE_LINES as isize)),
            Motion::PageDown => self.vertical(caret, PAGE_LINES as isize),
            Motion::WordLeft => self.word_left(caret),
            Motion::WordRight => self.word_right(caret),
        }
    }

    fn line_start(&self, offset: Offset) -> Offset {
        if self.len_chars() == 0 {
            return 0;
        }
        let line = self.rope.char_to_line(offset.min(self.len_chars()));
        self.rope.line_to_char(line)
    }

    fn line_end(&self, offset: Offset) -> Offset {
        if self.len_chars() == 0 {
            return 0;
        }
        let line = self.rope.char_to_line(offset.min(self.len_chars()));
        let start = self.rope.line_to_char(line);
        let line_text = self.rope.line(line);
        let mut end = start + line_text.len_chars();
        if end > start {
            let last = self.rope.char(end - 1);
            if last == '\n' {
                end -= 1;
                if end > start && self.rope.char(end - 1) == '\r' {
                    end -= 1;
                }
            } else if last == '\r' {
                end -= 1;
            }
        }
        end
    }

    fn vertical(&mut self, caret: Offset, delta: isize) -> Offset {
        let line = if self.len_chars() == 0 {
            0
        } else {
            self.rope.char_to_line(caret.min(self.len_chars()))
        };
        let column = self
            .preferred_column
            .unwrap_or_else(|| caret - self.line_start(caret));
        self.preferred_column = Some(column);
        let target = if delta < 0 {
            line.saturating_sub((-delta) as usize)
        } else {
            (line + delta as usize).min(self.rope.len_lines().saturating_sub(1))
        };
        let start = self.rope.line_to_char(target);
        let end = self.line_end(start);
        (start + column).min(end)
    }

    fn is_word(ch: char) -> bool {
        ch.is_ascii_alphanumeric() || ch == '_' || ch.is_alphabetic()
    }

    fn word_right(&self, caret: Offset) -> Offset {
        let chars: Vec<char> = self.text().chars().collect();
        let mut i = caret.min(chars.len());
        if i >= chars.len() {
            return chars.len();
        }
        if Self::is_word(chars[i]) {
            while i < chars.len() && Self::is_word(chars[i]) {
                i += 1;
            }
        } else if !chars[i].is_whitespace() {
            while i < chars.len() && !Self::is_word(chars[i]) && !chars[i].is_whitespace() {
                i += 1;
            }
        } else {
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            while i < chars.len() && Self::is_word(chars[i]) {
                i += 1;
            }
        }
        i
    }

    fn word_left(&self, caret: Offset) -> Offset {
        let chars: Vec<char> = self.text().chars().collect();
        let mut i = caret.min(chars.len());
        if i == 0 {
            return 0;
        }
        i -= 1;
        if Self::is_word(chars[i]) {
            while i > 0 && Self::is_word(chars[i - 1]) {
                i -= 1;
            }
        } else if chars[i].is_whitespace() {
            while i > 0 && chars[i - 1].is_whitespace() {
                i -= 1;
            }
            if i > 0 && Self::is_word(chars[i - 1]) {
                i -= 1;
                while i > 0 && Self::is_word(chars[i - 1]) {
                    i -= 1;
                }
            }
        } else {
            while i > 0 && !Self::is_word(chars[i - 1]) && !chars[i - 1].is_whitespace() {
                i -= 1;
            }
        }
        i
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

    fn push_edit(&mut self, edit: Edit) {
        if edit.coalesce_inserts {
            if let Some(last) = self.undo_stack.last_mut() {
                if last.coalesce_inserts
                    && last.inserted.chars().count() >= 1
                    && last.after.caret == edit.insert_start
                    && last.deleted.is_empty()
                    && edit.deleted.is_empty()
                {
                    last.inserted.push_str(&edit.inserted);
                    last.after = edit.after;
                    self.redo_stack.clear();
                    return;
                }
            }
        }
        self.undo_stack.push(edit);
        self.redo_stack.clear();
    }

    fn apply_inverse(&mut self, edit: &Edit) {
        if !edit.inserted.is_empty() {
            let start = edit.insert_start;
            let end = start + edit.inserted.chars().count();
            self.rope.remove(start..end);
        }
        if !edit.deleted.is_empty() {
            self.rope.insert(edit.delete_start, &edit.deleted);
        }
        self.selection = edit.before;
    }

    fn apply_forward(&mut self, edit: &Edit) {
        if !edit.deleted.is_empty() {
            let start = edit.delete_start;
            let end = start + edit.deleted.chars().count();
            self.rope.remove(start..end);
        }
        if !edit.inserted.is_empty() {
            self.rope.insert(edit.insert_start, &edit.inserted);
        }
        self.selection = edit.after;
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

    #[test]
    fn consecutive_single_char_inserts_undo_as_one_step() {
        let mut doc = Document::new();
        doc.insert("h");
        doc.insert("i");
        assert_eq!(doc.text(), "hi");
        assert!(doc.undo());
        assert_eq!(doc.text(), "");
        assert!(!doc.undo());
    }

    #[test]
    fn undo_then_new_edit_clears_redo() {
        let mut doc = Document::from_text("a");
        doc.set_caret(1);
        doc.insert("b");
        assert!(doc.undo());
        doc.insert("c");
        assert!(!doc.redo());
        assert_eq!(doc.text(), "ac");
    }

    #[test]
    fn delete_is_its_own_undo_step() {
        let mut doc = Document::from_text("ab");
        doc.set_caret(2);
        doc.delete_backward();
        doc.insert("c");
        assert!(doc.undo());
        assert_eq!(doc.text(), "a");
        assert!(doc.undo());
        assert_eq!(doc.text(), "ab");
    }

    #[test]
    fn arrows_and_home_end_move_caret() {
        let mut doc = Document::from_text("ab\nc");
        doc.set_caret(0);
        doc.move_caret(crate::Motion::Right, false);
        assert_eq!(doc.selection().caret, 1);
        doc.move_caret(crate::Motion::End, false);
        assert_eq!(doc.selection().caret, 2);
        doc.move_caret(crate::Motion::Down, false);
        assert_eq!(doc.selection().caret, 4);
        doc.move_caret(crate::Motion::Home, false);
        assert_eq!(doc.selection().caret, 3);
        assert_eq!(doc.line_column(), (2, 1));
    }

    #[test]
    fn shift_extends_selection() {
        let mut doc = Document::from_text("abcd");
        doc.set_caret(1);
        doc.move_caret(crate::Motion::Right, true);
        doc.move_caret(crate::Motion::Right, true);
        assert_eq!(
            doc.selection(),
            Selection {
                anchor: 1,
                caret: 3
            }
        );
        assert_eq!(doc.selected_text(), "bc");
    }

    #[test]
    fn word_right_skips_a_word() {
        let mut doc = Document::from_text("foo bar");
        doc.set_caret(0);
        doc.move_caret(crate::Motion::WordRight, false);
        assert_eq!(doc.selection().caret, 3);
    }
}
