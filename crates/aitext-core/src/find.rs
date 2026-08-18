use crate::document::Document;
use crate::selection::{Offset, Selection};
use crate::undo::Edit;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Forward,
    Backward,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FindQuery {
    pub text: String,
    pub match_case: bool,
    pub whole_word: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Match {
    pub start: Offset,
    pub end: Offset,
}

fn chars(text: &str) -> Vec<char> {
    text.chars().collect()
}

fn equal_at(hay: &[char], start: usize, needle: &[char], match_case: bool) -> bool {
    if start + needle.len() > hay.len() {
        return false;
    }
    for (i, n) in needle.iter().enumerate() {
        let h = hay[start + i];
        if match_case {
            if h != *n {
                return false;
            }
        } else if !h.to_lowercase().eq(n.to_lowercase()) {
            return false;
        }
    }
    true
}

fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

fn is_whole_word(hay: &[char], start: usize, end: usize) -> bool {
    let left_ok = start == 0 || !is_word_char(hay[start - 1]);
    let right_ok = end == hay.len() || !is_word_char(hay[end]);
    left_ok && right_ok
}

impl Document {
    pub fn find(&self, query: &FindQuery, from: Offset, direction: Direction) -> Option<Match> {
        if query.text.is_empty() {
            return None;
        }
        let hay = chars(&self.text());
        let needle = chars(&query.text);
        if needle.is_empty() || hay.len() < needle.len() {
            return None;
        }
        match direction {
            Direction::Forward => {
                let mut i = from.min(hay.len());
                while i + needle.len() <= hay.len() {
                    if equal_at(&hay, i, &needle, query.match_case)
                        && (!query.whole_word || is_whole_word(&hay, i, i + needle.len()))
                    {
                        return Some(Match {
                            start: i,
                            end: i + needle.len(),
                        });
                    }
                    i += 1;
                }
                None
            }
            Direction::Backward => {
                let mut i = from.min(hay.len());
                if i >= needle.len() {
                    i -= needle.len();
                    loop {
                        if equal_at(&hay, i, &needle, query.match_case)
                            && (!query.whole_word || is_whole_word(&hay, i, i + needle.len()))
                        {
                            return Some(Match {
                                start: i,
                                end: i + needle.len(),
                            });
                        }
                        if i == 0 {
                            break;
                        }
                        i -= 1;
                    }
                }
                None
            }
        }
    }

    pub fn find_all(&self, query: &FindQuery) -> Vec<Match> {
        let mut out = Vec::new();
        let mut from = 0;
        while let Some(m) = self.find(query, from, Direction::Forward) {
            out.push(m);
            from = if m.end > m.start { m.end } else { m.start + 1 };
        }
        out
    }

    pub fn replace_current(&mut self, query: &FindQuery, replacement: &str) -> bool {
        if self.is_readonly() {
            return false;
        }
        let sel = self.selection();
        let current = if !sel.is_empty() {
            Some(Match {
                start: sel.start(),
                end: sel.end(),
            })
        } else {
            None
        };
        let is_match = current
            .and_then(|m| {
                self.find(query, m.start, Direction::Forward)
                    .filter(|found| found.start == m.start && found.end == m.end)
            })
            .is_some();
        let target = if is_match {
            current
        } else {
            self.find(query, self.selection().caret, Direction::Forward)
        };
        let Some(m) = target else {
            return false;
        };
        self.set_selection(Selection {
            anchor: m.start,
            caret: m.end,
        });
        self.insert(replacement);
        true
    }

    pub fn replace_all(&mut self, query: &FindQuery, replacement: &str) -> usize {
        if self.is_readonly() {
            return 0;
        }
        let matches = self.find_all(query);
        if matches.is_empty() {
            return 0;
        }
        let before = self.selection();
        let original = self.text();
        let chars: Vec<char> = original.chars().collect();
        let mut out = String::new();
        let mut last = 0usize;
        for m in &matches {
            out.extend(chars[last..m.start].iter().copied());
            out.push_str(replacement);
            last = m.end;
        }
        out.extend(chars[last..].iter().copied());
        let count = matches.len();
        self.apply_full_replace(original, out, before);
        count
    }

    fn apply_full_replace(&mut self, old: String, new: String, before: Selection) {
        self.rope_replace_all(&new);
        let after = Selection {
            anchor: self.len_chars(),
            caret: self.len_chars(),
        };
        self.set_selection(after);
        self.push_public_edit(Edit {
            delete_start: 0,
            deleted: old,
            insert_start: 0,
            inserted: new,
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

    #[test]
    fn find_next_and_previous() {
        let doc = Document::from_text("one two one");
        let q = FindQuery {
            text: "one".into(),
            match_case: false,
            whole_word: false,
        };
        let first = doc.find(&q, 0, Direction::Forward).unwrap();
        assert_eq!(first, Match { start: 0, end: 3 });
        let second = doc.find(&q, first.end, Direction::Forward).unwrap();
        assert_eq!(second, Match { start: 8, end: 11 });
        let back = doc.find(&q, second.start, Direction::Backward).unwrap();
        assert_eq!(back, first);
    }

    #[test]
    fn whole_word_and_case() {
        let doc = Document::from_text("One someone ONE");
        let q = FindQuery {
            text: "one".into(),
            match_case: true,
            whole_word: true,
        };
        assert!(doc.find(&q, 0, Direction::Forward).is_none());
        let q = FindQuery {
            text: "ONE".into(),
            match_case: true,
            whole_word: true,
        };
        assert_eq!(doc.find(&q, 0, Direction::Forward).unwrap().start, 12);
    }

    #[test]
    fn replace_all_is_one_undo_step() {
        let mut doc = Document::from_text("a a a");
        let q = FindQuery {
            text: "a".into(),
            match_case: true,
            whole_word: true,
        };
        assert_eq!(doc.replace_all(&q, "b"), 3);
        assert_eq!(doc.text(), "b b b");
        assert!(doc.undo());
        assert_eq!(doc.text(), "a a a");
    }
}
