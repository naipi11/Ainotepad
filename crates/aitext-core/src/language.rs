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
    JavaScript,
    TypeScript,
    PowerShell,
    Batch,
    Ini,
}

pub fn language_from_path(path: &str) -> LanguageId {
    let lower = path.to_ascii_lowercase();
    let ext = if let Some((_, after)) = lower.rsplit_once('.') {
        after.rsplit(['/', '\\']).next().unwrap_or(after)
    } else {
        ""
    };
    match ext {
        "md" => LanguageId::Markdown,
        "json" => LanguageId::Json,
        "toml" => LanguageId::Toml,
        "rs" => LanguageId::Rust,
        "py" => LanguageId::Python,
        "c" | "h" => LanguageId::C,
        "cpp" | "cc" | "cxx" | "hpp" => LanguageId::Cpp,
        "js" | "mjs" | "cjs" => LanguageId::JavaScript,
        "ts" | "tsx" => LanguageId::TypeScript,
        "ps1" | "psm1" => LanguageId::PowerShell,
        "bat" | "cmd" => LanguageId::Batch,
        "ini" | "cfg" => LanguageId::Ini,
        _ => LanguageId::PlainText,
    }
}
