use crate::highlight::{Token, TokenKind};
use crate::lexers::{
    is_ident_start, push, scan_block_comment, scan_ident_or_keyword, scan_line_comment, scan_number,
    scan_string,
};

const KEYWORDS: &[&str] = &[
    "function", "let", "const", "var", "return", "if", "else", "for", "while", "class", "import",
    "export", "async", "await", "type", "interface",
];

pub fn lex(text: &str) -> Vec<Token> {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut tokens = Vec::new();
    while i < chars.len() {
        let c = chars[i];
        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
            scan_line_comment(&chars, &mut i, &mut tokens, 2);
            continue;
        }
        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
            scan_block_comment(&chars, &mut i, &mut tokens);
            continue;
        }
        if c == '"' || c == '\'' || c == '`' {
            scan_string(&chars, &mut i, &mut tokens, c);
            continue;
        }
        if c.is_ascii_digit() {
            scan_number(&chars, &mut i, &mut tokens);
            continue;
        }
        if is_ident_start(c) {
            scan_ident_or_keyword(&chars, &mut i, &mut tokens, KEYWORDS);
            continue;
        }
        if matches!(c, '{' | '}' | '(' | ')' | '[' | ']' | ';' | ',' | '.' | ':') {
            push(&mut tokens, i, i + 1, TokenKind::Punct);
            i += 1;
            continue;
        }
        i += 1;
    }
    tokens
}
