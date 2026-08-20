pub mod batch;
pub mod c_family;
pub mod ini;
pub mod javascript;
pub mod json;
pub mod markdown;
pub mod plain;
pub mod powershell;
pub mod python;
pub mod rust;
pub mod toml;

use crate::highlight::{Token, TokenKind};

pub fn take_while(chars: &[char], i: &mut usize, pred: impl Fn(char) -> bool) -> (usize, usize) {
    let start = *i;
    while *i < chars.len() && pred(chars[*i]) {
        *i += 1;
    }
    (start, *i)
}
pub fn push(tokens: &mut Vec<Token>, start: usize, end: usize, kind: TokenKind) {
    if end > start {
        tokens.push(Token { start, end, kind });
    }
}
pub fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

pub fn is_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

pub fn scan_line_comment(
    chars: &[char],
    i: &mut usize,
    tokens: &mut Vec<Token>,
    starter_len: usize,
) {
    let start = *i;
    *i += starter_len;
    while *i < chars.len() && chars[*i] != '\n' {
        *i += 1;
    }
    push(tokens, start, *i, TokenKind::Comment);
}

pub fn scan_block_comment(chars: &[char], i: &mut usize, tokens: &mut Vec<Token>) {
    let start = *i;
    *i += 2;
    while *i + 1 < chars.len() && !(chars[*i] == '*' && chars[*i + 1] == '/') {
        *i += 1;
    }
    if *i + 1 < chars.len() {
        *i += 2;
    } else {
        *i = chars.len();
    }
    push(tokens, start, *i, TokenKind::Comment);
}

pub fn scan_string(chars: &[char], i: &mut usize, tokens: &mut Vec<Token>, quote: char) {
    let start = *i;
    *i += 1;
    while *i < chars.len() {
        if chars[*i] == '\\' && *i + 1 < chars.len() {
            *i += 2;
            continue;
        }
        if chars[*i] == quote {
            *i += 1;
            break;
        }
        *i += 1;
    }
    push(tokens, start, *i, TokenKind::String);
}

pub fn scan_number(chars: &[char], i: &mut usize, tokens: &mut Vec<Token>) {
    let (start, end) = take_while(chars, i, |c| c.is_ascii_digit() || c == '.' || c == '_');
    push(tokens, start, end, TokenKind::Number);
}

pub fn scan_ident_or_keyword(
    chars: &[char],
    i: &mut usize,
    tokens: &mut Vec<Token>,
    keywords: &[&str],
) {
    scan_ident_classified(chars, i, tokens, keywords, &[], &[]);
}

pub fn scan_ident_classified(
    chars: &[char],
    i: &mut usize,
    tokens: &mut Vec<Token>,
    keywords: &[&str],
    controls: &[&str],
    types: &[&str],
) {
    let (start, end) = take_while(chars, i, is_ident_continue);
    let word: String = chars[start..end].iter().collect();
    let mut kind = if controls.contains(&word.as_str()) {
        TokenKind::Control
    } else if types.contains(&word.as_str()) {
        TokenKind::Type
    } else if keywords.contains(&word.as_str()) {
        TokenKind::Keyword
    } else {
        TokenKind::Ident
    };
    if kind == TokenKind::Ident {
        let mut j = end;
        while j < chars.len() && chars[j].is_whitespace() {
            j += 1;
        }
        if j < chars.len() && chars[j] == char::from_u32(40).unwrap() {
            kind = TokenKind::Function;
        } else if word
            .chars()
            .next()
            .map(|c| c.is_ascii_uppercase())
            .unwrap_or(false)
        {
            kind = TokenKind::Type;
        }
    }
    push(tokens, start, end, kind);
}
