#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LanguageId {
    PlainText,
    Markdown,
    Json,
    Toml,
    Rust,
    Python,
    C,
    Cpp,
    CSharp,
    JavaScript,
    TypeScript,
    Html,
    Css,
    PowerShell,
    Batch,
    Ini,
}

impl LanguageId {
    pub const ALL: &'static [Self] = &[
        Self::Markdown,
        Self::PlainText,
        Self::C,
        Self::Cpp,
        Self::CSharp,
        Self::Python,
        Self::Rust,
        Self::JavaScript,
        Self::TypeScript,
        Self::Html,
        Self::Css,
        Self::Json,
        Self::Toml,
        Self::PowerShell,
        Self::Batch,
        Self::Ini,
    ];
}

pub fn language_from_path(path: &str) -> LanguageId {
    let lower = path.to_ascii_lowercase();
    let ext = if let Some((_, after)) = lower.rsplit_once('.') {
        after.rsplit(['/', '\\']).next().unwrap_or(after)
    } else {
        ""
    };
    match ext {
        "md" | "markdown" | "mdown" => LanguageId::Markdown,
        "txt" | "text" | "log" => LanguageId::PlainText,
        "json" | "jsonc" => LanguageId::Json,
        "toml" => LanguageId::Toml,
        "rs" => LanguageId::Rust,
        "py" | "pyw" => LanguageId::Python,
        "c" | "h" => LanguageId::C,
        "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" => LanguageId::Cpp,
        "cs" => LanguageId::CSharp,
        "js" | "mjs" | "cjs" => LanguageId::JavaScript,
        "ts" | "tsx" => LanguageId::TypeScript,
        "html" | "htm" => LanguageId::Html,
        "css" => LanguageId::Css,
        "ps1" | "psm1" | "psd1" => LanguageId::PowerShell,
        "bat" | "cmd" => LanguageId::Batch,
        "ini" | "cfg" | "conf" => LanguageId::Ini,
        _ => LanguageId::Markdown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;

    #[test]
    fn new_document_defaults_to_markdown() {
        assert_eq!(Document::new().language(), LanguageId::Markdown);
    }

    #[test]
    fn mainstream_extensions_are_case_insensitive() {
        assert_eq!(language_from_path("main.CPP"), LanguageId::Cpp);
        assert_eq!(language_from_path("Program.Cs"), LanguageId::CSharp);
        assert_eq!(language_from_path("page.HTML"), LanguageId::Html);
        assert_eq!(language_from_path("theme.CSS"), LanguageId::Css);
        assert_eq!(language_from_path("notes.TXT"), LanguageId::PlainText);
        assert_eq!(language_from_path("note.unknown"), LanguageId::Markdown);
    }
}
