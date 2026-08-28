use crate::language::LanguageId;
use crate::lexers;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Text,
    Comment,
    String,
    Number,
    Keyword,
    Control,
    Type,
    Function,
    Ident,
    Punct,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Token {
    pub start: usize,
    pub end: usize,
    pub kind: TokenKind,
}

pub fn highlight(text: &str, language: LanguageId) -> Vec<Token> {
    if text.is_empty() {
        return Vec::new();
    }
    let raw = std::panic::catch_unwind(|| match language {
        LanguageId::PlainText => lexers::plain::lex(text),
        LanguageId::Markdown => lexers::markdown::lex(text),
        LanguageId::Json => lexers::json::lex(text),
        LanguageId::Toml => lexers::toml::lex(text),
        LanguageId::Rust => lexers::rust::lex(text),
        LanguageId::Python => lexers::python::lex(text),
        LanguageId::C | LanguageId::Cpp => lexers::c_family::lex(text),
        LanguageId::CSharp => lexers::csharp::lex(text),
        LanguageId::JavaScript | LanguageId::TypeScript => lexers::javascript::lex(text),
        LanguageId::Html => lexers::html::lex(text),
        LanguageId::Css => lexers::css::lex(text),
        LanguageId::PowerShell => lexers::powershell::lex(text),
        LanguageId::Batch => lexers::batch::lex(text),
        LanguageId::Ini => lexers::ini::lex(text),
    })
    .unwrap_or_default();
    fill_gaps(text, raw)
}

fn fill_gaps(text: &str, tokens: Vec<Token>) -> Vec<Token> {
    let len = text.chars().count();
    if tokens.is_empty() {
        return vec![Token {
            start: 0,
            end: len,
            kind: TokenKind::Text,
        }];
    }
    let mut out = Vec::new();
    let mut cursor = 0;
    for token in tokens {
        if token.start > cursor {
            out.push(Token {
                start: cursor,
                end: token.start,
                kind: TokenKind::Text,
            });
        }
        if token.end > token.start {
            out.push(token);
            cursor = token.end;
        }
    }
    if cursor < len {
        out.push(Token {
            start: cursor,
            end: len,
            kind: TokenKind::Text,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_or_plain_is_one_text_span() {
        let tokens = highlight("hello", LanguageId::PlainText);
        assert_eq!(
            tokens,
            vec![Token {
                start: 0,
                end: 5,
                kind: TokenKind::Text
            }]
        );
    }

    #[test]
    fn rust_highlights_fn_and_comment() {
        let tokens = highlight("fn main() { // hi", LanguageId::Rust);
        assert!(tokens
            .iter()
            .any(|t| t.kind == TokenKind::Keyword && t.start == 0 && t.end == 2));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Function));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Comment));
    }

    #[test]
    fn json_string_and_keyword() {
        let tokens = highlight("{\"ok\": true}", LanguageId::Json);
        assert!(tokens.iter().any(|t| t.kind == TokenKind::String));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Keyword));
    }

    #[test]
    fn python_highlights_print_and_number() {
        let tokens = highlight("a = 1\nprint(c)", LanguageId::Python);
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Number));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Function));
    }

    #[test]
    fn csharp_highlights_type_method_and_string() {
        let tokens = highlight(
            "public class Demo { string name = \"A\"; }",
            LanguageId::CSharp,
        );
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Keyword));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Type));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::String));
    }

    #[test]
    fn html_highlights_tag_attribute_and_comment() {
        let tokens = highlight("<!-- x --><div class=\"app\">Hi</div>", LanguageId::Html);
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Comment));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Keyword));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::String));
    }

    #[test]
    fn css_highlights_selector_property_and_number() {
        let tokens = highlight(".app { color: #fff; margin: 4px; }", LanguageId::Css);
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Ident));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Keyword));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Number));
    }

    #[test]
    fn tokens_cover_the_whole_document() {
        let text = "let x = 1; // c";
        let tokens = highlight(text, LanguageId::Rust);
        assert_eq!(tokens.first().unwrap().start, 0);
        assert_eq!(tokens.last().unwrap().end, text.chars().count());
        for pair in tokens.windows(2) {
            assert_eq!(pair[0].end, pair[1].start);
        }
    }

    #[test]
    fn new_lexers_cover_their_complete_inputs() {
        for (text, language) in [
            (
                "public class Demo { string name = \"A\"; }",
                LanguageId::CSharp,
            ),
            ("<div class=\"app\">Hi</div>", LanguageId::Html),
            (".app { color: #fff; margin: 4px; }", LanguageId::Css),
        ] {
            let tokens = highlight(text, language);
            assert_eq!(tokens.first().unwrap().start, 0);
            assert_eq!(tokens.last().unwrap().end, text.chars().count());
            for pair in tokens.windows(2) {
                assert_eq!(pair[0].end, pair[1].start);
            }
        }
    }

    #[test]
    fn new_lexers_handle_unterminated_constructs() {
        assert!(!highlight("public class Demo { string name = \"A", LanguageId::CSharp).is_empty());
        assert!(!highlight("<div class=\"app\"", LanguageId::Html).is_empty());
        assert!(!highlight(".app { color: \"#fff", LanguageId::Css).is_empty());
    }
}
