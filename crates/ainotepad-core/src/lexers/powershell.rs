use crate::highlight::{Token, TokenKind};
use crate::lexers::{push, scan_line_comment, scan_number, scan_string, take_while};

const KEYWORDS: &[&str] = &[
    "function", "param", "if", "else", "foreach", "while", "return", "$true", "$false",
];

pub fn lex(text: &str) -> Vec<Token> {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut tokens = Vec::new();
    while i < chars.len() {
        let c = chars[i];
        if c == '#' {
            scan_line_comment(&chars, &mut i, &mut tokens, 1);
            continue;
        }
        if c == '"' || c == '\'' {
            scan_string(&chars, &mut i, &mut tokens, c);
            continue;
        }
        if c.is_ascii_digit() {
            scan_number(&chars, &mut i, &mut tokens);
            continue;
        }
        if c == '$' || c.is_ascii_alphabetic() || c == '_' {
            let (start, end) = take_while(&chars, &mut i, |ch| {
                ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'
            });
            let word: String = chars[start..end].iter().collect();
            let kind = if KEYWORDS.iter().any(|k| k.eq_ignore_ascii_case(&word)) {
                TokenKind::Keyword
            } else {
                TokenKind::Ident
            };
            push(&mut tokens, start, end, kind);
            continue;
        }
        i += 1;
    }
    tokens
}
