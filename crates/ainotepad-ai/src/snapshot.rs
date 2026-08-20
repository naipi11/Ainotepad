use ainotepad_core::{Document, LanguageId};

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
    let text = doc.text();
    let chars: Vec<char> = text.chars().collect();
    let caret = doc.selection().caret.min(chars.len());
    let prefix_start = caret.saturating_sub(PREFIX_CONTEXT_CHARS);
    let suffix_end = (caret + SUFFIX_CONTEXT_CHARS).min(chars.len());
    CompletionSnapshot {
        document_id: doc.id(),
        prefix: chars[prefix_start..caret].iter().collect(),
        suffix: chars[caret..suffix_end].iter().collect(),
        file_name: doc.display_name(),
        language: language_name(doc.language()).into(),
        generation,
    }
}
