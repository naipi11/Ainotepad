use crate::highlight::{Token, TokenKind};
use crate::lexers::{push, take_while};

const KEYWORDS: &[&str] = &["echo", "set", "if", "else", "goto", "call", "rem"];

pub fn lex(text: &str) -> Vec<Token> {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut tokens = Vec::new();
    while i < chars.len() {
        if i + 1 < chars.len() && chars[i] == ':' && chars[i + 1] == ':' {
            let start = i;
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            push(&mut tokens, start, i, TokenKind::Comment);
            continue;
        }
        match chars[i] {
            c if c.is_ascii_alphabetic() => {
                let (start, end) = take_while(&chars, &mut i, |ch| ch.is_ascii_alphanumeric());
                let word: String = chars[start..end].iter().collect();
                if word.eq_ignore_ascii_case("rem") {
                    while i < chars.len() && chars[i] != '\n' {
                        i += 1;
                    }
                    push(&mut tokens, start, i, TokenKind::Comment);
                } else {
                    let kind = if KEYWORDS.iter().any(|k| k.eq_ignore_ascii_case(&word)) {
                        TokenKind::Keyword
                    } else {
                        TokenKind::Ident
                    };
                    push(&mut tokens, start, end, kind);
                }
            }
            _ => i += 1,
        }
    }
    tokens
}
