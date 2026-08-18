use crate::document::Document;
use crate::language::language_from_path;

#[derive(Debug, Default)]
pub struct Workspace {
    documents: Vec<Document>,
    current: Option<u64>,
    next_id: u64,
    next_untitled: u32,
}

impl Workspace {
    pub fn new() -> Self {
        Self {
            documents: Vec::new(),
            current: None,
            next_id: 1,
            next_untitled: 1,
        }
    }

    pub fn documents(&self) -> impl Iterator<Item = &Document> {
        self.documents.iter()
    }

    pub fn current_id(&self) -> Option<u64> {
        self.current
    }

    pub fn current(&self) -> Option<&Document> {
        self.current.and_then(|id| self.get(id))
    }

    pub fn current_mut(&mut self) -> Option<&mut Document> {
        let id = self.current?;
        self.get_mut(id)
    }

    pub fn get(&self, id: u64) -> Option<&Document> {
        self.documents.iter().find(|doc| doc.id() == id)
    }

    pub fn get_mut(&mut self, id: u64) -> Option<&mut Document> {
        self.documents.iter_mut().find(|doc| doc.id() == id)
    }

    pub fn new_untitled(&mut self) -> u64 {
        let number = self.next_untitled;
        self.next_untitled += 1;
        let mut doc = Document::new();
        doc.set_untitled_number(number);
        self.add_document(doc)
    }

    pub fn add_document(&mut self, mut document: Document) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        document.set_id(id);
        if let Some(path) = document.path() {
            document.set_language(language_from_path(path));
        }
        self.documents.push(document);
        self.current = Some(id);
        id
    }

    pub fn set_current(&mut self, id: u64) -> bool {
        if self.get(id).is_some() {
            self.current = Some(id);
            true
        } else {
            false
        }
    }

    pub fn next_tab(&mut self) {
        if self.documents.is_empty() {
            return;
        }
        let Some(current) = self.current else {
            self.current = Some(self.documents[0].id());
            return;
        };
        let idx = self
            .documents
            .iter()
            .position(|d| d.id() == current)
            .unwrap_or(0);
        let next = (idx + 1) % self.documents.len();
        self.current = Some(self.documents[next].id());
    }

    pub fn prev_tab(&mut self) {
        if self.documents.is_empty() {
            return;
        }
        let Some(current) = self.current else {
            self.current = Some(self.documents[0].id());
            return;
        };
        let idx = self
            .documents
            .iter()
            .position(|d| d.id() == current)
            .unwrap_or(0);
        let prev = if idx == 0 {
            self.documents.len() - 1
        } else {
            idx - 1
        };
        self.current = Some(self.documents[prev].id());
    }

    pub fn close(&mut self, id: u64) -> Option<Document> {
        let idx = self.documents.iter().position(|d| d.id() == id)?;
        let doc = self.documents.remove(idx);
        if self.current == Some(id) {
            self.current = self
                .documents
                .get(idx)
                .or_else(|| self.documents.get(idx.saturating_sub(1)))
                .map(|d| d.id());
        }
        Some(doc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language::LanguageId;

    #[test]
    fn untitled_names_increment_and_do_not_reuse() {
        let mut ws = Workspace::new();
        let a = ws.new_untitled();
        let b = ws.new_untitled();
        assert_eq!(ws.get(a).unwrap().display_name(), "Untitled-1");
        assert_eq!(ws.get(b).unwrap().display_name(), "Untitled-2");
        ws.close(a);
        let c = ws.new_untitled();
        assert_eq!(ws.get(c).unwrap().display_name(), "Untitled-3");
    }

    #[test]
    fn language_from_common_extensions() {
        assert_eq!(language_from_path("main.rs"), LanguageId::Rust);
        assert_eq!(language_from_path("C:\\a\\b.py"), LanguageId::Python);
        assert_eq!(language_from_path("note.TXT"), LanguageId::PlainText);
    }

    #[test]
    fn close_selects_neighbor() {
        let mut ws = Workspace::new();
        let a = ws.new_untitled();
        let b = ws.new_untitled();
        let c = ws.new_untitled();
        ws.set_current(b);
        ws.close(b);
        assert_eq!(ws.current_id(), Some(c));
        ws.close(c);
        assert_eq!(ws.current_id(), Some(a));
    }
}
