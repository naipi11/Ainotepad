use crate::highlight::Token;
use crate::lexers::{
    is_ident_start, push, scan_block_comment, scan_ident_or_keyword, scan_line_comment,
    scan_number, scan_string,
};
use crate::highlight::TokenKind;

const KEYWORDS: &[&str] = &[
    "fn", "let", "mut", "pub", "struct", "enum", "impl", "use", "crate", "mod", "if", "else",
    "match", "return", "async", "await",
];

pub fn lex(text: &str) -> Vec<Token> {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut tokens = Vec::new();
    while i < chars.len() {
        match chars[i] {
            '/' if i + 1 < chars.len() && chars[i + 1] == '/' => {
                scan_line_comment(&chars, &mut i, &mut tokens, 2)
            }
            '/' if i + 1 < chars.len() && chars[i + 1] == '*' => {
                scan_block_comment(&chars, &mut i, &mut tokens)
            }
            '"' => scan_string(&chars, &mut i, &mut tokens, '"'),
            c if c.is_ascii_digit() => scan_number(&chars, &mut i, &mut tokens),
            c if is_ident_start(c) => {
                scan_ident_or_keyword(&chars, &mut i, &mut tokens, KEYWORDS)
            }
            '{' | '}' | '(' | ')' | '[' | ']' | ';' | ',' | '.' | ':' | '=' | '+' | '-' | '*'
            | '!' | '?' | '<' | '>' => {
                push(&mut tokens, i, i + 1, TokenKind::Punct);
                i += 1;
            }
            _ => i += 1,
        }
    }
    tokens
}
