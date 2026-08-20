use crate::highlight::{Token, TokenKind};
use crate::lexers::{push, scan_number, scan_string, take_while};

const KEYWORDS: &[&str] = &["true", "false", "null"];

pub fn lex(text: &str) -> Vec<Token> {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut tokens = Vec::new();
    while i < chars.len() {
        match chars[i] {
            '"' => scan_string(&chars, &mut i, &mut tokens, '"'),
            c if c.is_ascii_digit() || c == '-' => scan_number(&chars, &mut i, &mut tokens),
            '{' | '}' | '[' | ']' | ':' | ',' => {
                push(&mut tokens, i, i + 1, TokenKind::Punct);
                i += 1;
            }
            c if c.is_ascii_alphabetic() => {
                let (start, end) = take_while(&chars, &mut i, |ch| ch.is_ascii_alphabetic());
                let word: String = chars[start..end].iter().collect();
                let kind = if KEYWORDS.contains(&word.as_str()) {
                    TokenKind::Keyword
                } else {
                    TokenKind::Text
                };
                push(&mut tokens, start, end, kind);
            }
            _ => i += 1,
        }
    }
    tokens
}
