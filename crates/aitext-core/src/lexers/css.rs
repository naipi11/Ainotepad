use crate::highlight::{Token, TokenKind};
use crate::lexers::{is_ident_start, push, scan_block_comment, scan_number, scan_string};

pub fn lex(text: &str) -> Vec<Token> {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut tokens = Vec::new();
    while i < chars.len() {
        if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '*' {
            scan_block_comment(&chars, &mut i, &mut tokens);
            continue;
        }
        if chars[i] == '"' || chars[i] == '\'' {
            let quote = chars[i];
            scan_string(&chars, &mut i, &mut tokens, quote);
            continue;
        }
        if chars[i] == '#' {
            let start = i;
            i += 1;
            while i < chars.len() && chars[i].is_ascii_hexdigit() {
                i += 1;
            }
            if i > start + 1 {
                push(&mut tokens, start, i, TokenKind::Number);
            }
            continue;
        }
        if chars[i].is_ascii_digit() {
            scan_number(&chars, &mut i, &mut tokens);
            continue;
        }
        if is_ident_start(chars[i]) || chars[i] == '-' {
            let start = i;
            i += 1;
            while i < chars.len()
                && (chars[i].is_ascii_alphanumeric() || chars[i] == '_' || chars[i] == '-')
            {
                i += 1;
            }
            let mut next = i;
            while next < chars.len() && chars[next].is_whitespace() {
                next += 1;
            }
            let kind = if next < chars.len() && chars[next] == ':' {
                TokenKind::Keyword
            } else {
                TokenKind::Ident
            };
            push(&mut tokens, start, i, kind);
            continue;
        }
        if matches!(chars[i], '{' | '}' | ':' | ';' | ',' | '(' | ')') {
            push(&mut tokens, i, i + 1, TokenKind::Punct);
        }
        i += 1;
    }
    tokens
}
