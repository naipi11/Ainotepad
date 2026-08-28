use aitext_core::{Document, LanguageId};

pub const PREFIX_CONTEXT_CHARS: usize = 4000;
pub const SUFFIX_CONTEXT_CHARS: usize = 500;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletionSnapshot {
    pub document_id: u64,
    pub prefix: String,
    pub suffix: String,
    pub file_name: String,
    pub language: String,
    pub generation: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletionMode {
    Code,
    Markdown,
    PlainText,
}

impl CompletionMode {
    pub fn from_language(language: &str) -> Self {
        match language {
            "markdown" => Self::Markdown,
            "plain" => Self::PlainText,
            _ => Self::Code,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletionContext {
    pub document_id: u64,
    pub prefix: String,
    pub suffix: String,
    pub file_name: String,
    pub language: String,
    pub generation: u64,
    pub current_line: String,
    pub indentation: String,
}

impl CompletionContext {
    fn from_snapshot(snapshot: CompletionSnapshot) -> Self {
        let current_line = snapshot
            .prefix
            .rsplit('\n')
            .next()
            .unwrap_or_default()
            .to_string();
        let indentation = current_line
            .chars()
            .take_while(|ch| matches!(ch, ' ' | '\t'))
            .collect();
        Self {
            document_id: snapshot.document_id,
            prefix: snapshot.prefix,
            suffix: snapshot.suffix,
            file_name: snapshot.file_name,
            language: snapshot.language,
            generation: snapshot.generation,
            current_line,
            indentation,
        }
    }

    pub fn snapshot(&self) -> CompletionSnapshot {
        CompletionSnapshot {
            document_id: self.document_id,
            prefix: self.prefix.clone(),
            suffix: self.suffix.clone(),
            file_name: self.file_name.clone(),
            language: self.language.clone(),
            generation: self.generation,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletionRequest {
    pub context: CompletionContext,
    pub mode: CompletionMode,
}

impl CompletionRequest {
    pub fn from_snapshot(snapshot: &CompletionSnapshot) -> Self {
        let mode = CompletionMode::from_language(&snapshot.language);
        Self {
            context: CompletionContext::from_snapshot(snapshot.clone()),
            mode,
        }
    }

    pub fn from_document(doc: &Document, generation: u64) -> Self {
        Self::from_snapshot(&take_snapshot(doc, generation))
    }
}

pub fn language_name(id: LanguageId) -> &'static str {
    match id {
        LanguageId::PlainText => "plain",
        LanguageId::Markdown => "markdown",
        LanguageId::Json => "json",
        LanguageId::Toml => "toml",
        LanguageId::Rust => "rust",
        LanguageId::Python => "python",
        LanguageId::C => "c",
        LanguageId::Cpp => "cpp",
        LanguageId::CSharp => "csharp",
        LanguageId::JavaScript => "javascript",
        LanguageId::TypeScript => "typescript",
        LanguageId::Html => "html",
        LanguageId::Css => "css",
        LanguageId::PowerShell => "powershell",
        LanguageId::Batch => "batch",
        LanguageId::Ini => "ini",
    }
}

pub fn take_snapshot(doc: &Document, generation: u64) -> CompletionSnapshot {
    let (prefix, suffix) = doc.text_window_around_caret(PREFIX_CONTEXT_CHARS, SUFFIX_CONTEXT_CHARS);
    CompletionSnapshot {
        document_id: doc.id(),
        prefix,
        suffix,
        file_name: doc.display_name(),
        language: language_name(doc.language()).into(),
        generation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aitext_core::{Document, LanguageId};

    #[test]
    fn completion_request_preserves_language_mode_line_and_indent() {
        let mut doc = Document::from_text("def hello():\n    print(");
        doc.set_language(LanguageId::Python);
        doc.set_caret(doc.len_chars());

        let request = CompletionRequest::from_document(&doc, 7);

        assert_eq!(request.mode, CompletionMode::Code);
        assert_eq!(request.context.current_line, "    print(");
        assert_eq!(request.context.indentation, "    ");
        assert_eq!(request.context.generation, 7);
    }

    #[test]
    fn markdown_and_plain_text_have_distinct_completion_modes() {
        let mut markdown = Document::from_text("# Notes\nWrite a");
        markdown.set_language(LanguageId::Markdown);
        let mut plain = Document::from_text("Write a");
        plain.set_language(LanguageId::PlainText);

        assert_eq!(
            CompletionRequest::from_document(&markdown, 0).mode,
            CompletionMode::Markdown
        );
        assert_eq!(
            CompletionRequest::from_document(&plain, 0).mode,
            CompletionMode::PlainText
        );
    }
}
