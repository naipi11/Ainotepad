use crate::highlight::{Token, TokenKind};
use crate::lexers::{push, scan_line_comment, take_while};

pub fn lex(text: &str) -> Vec<Token> {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut tokens = Vec::new();
    while i < chars.len() {
        match chars[i] {
            ';' | '#' => scan_line_comment(&chars, &mut i, &mut tokens, 1),
            '[' => {
                let start = i;
                while i < chars.len() && chars[i] != ']' && chars[i] != '\n' {
                    i += 1;
                }
                if i < chars.len() && chars[i] == ']' {
                    i += 1;
                }
                push(&mut tokens, start, i, TokenKind::Ident);
            }
            c if c.is_ascii_alphanumeric() || c == '_' => {
                let (start, end) = take_while(&chars, &mut i, |ch| {
                    ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.'
                });
                push(&mut tokens, start, end, TokenKind::Ident);
            }
            '=' => {
                push(&mut tokens, i, i + 1, TokenKind::Punct);
                i += 1;
                let start = i;
                while i < chars.len() && chars[i] != '\n' && chars[i] != ';' && chars[i] != '#' {
                    i += 1;
                }
                push(&mut tokens, start, i, TokenKind::String);
            }
            _ => i += 1,
        }
    }
    tokens
}
