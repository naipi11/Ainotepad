use crate::highlight::{Token, TokenKind};
use crate::lexers::{
    is_ident_start, push, scan_ident_or_keyword, scan_line_comment, scan_number, scan_string,
};

const KEYWORDS: &[&str] = &["true", "false"];

pub fn lex(text: &str) -> Vec<Token> {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut tokens = Vec::new();
    while i < chars.len() {
        match chars[i] {
            '#' => scan_line_comment(&chars, &mut i, &mut tokens, 1),
            '"' | '\'' => {
                let q = chars[i];
                scan_string(&chars, &mut i, &mut tokens, q);
            }
            c if c.is_ascii_digit() => scan_number(&chars, &mut i, &mut tokens),
            c if is_ident_start(c) => scan_ident_or_keyword(&chars, &mut i, &mut tokens, KEYWORDS),
            '[' | ']' | '{' | '}' | '=' | ',' | '.' => {
                push(&mut tokens, i, i + 1, TokenKind::Punct);
                i += 1;
            }
            _ => i += 1,
        }
    }
    tokens
}
