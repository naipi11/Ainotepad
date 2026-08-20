use crate::highlight::Token;
use crate::highlight::TokenKind;
use crate::lexers::{
    is_ident_start, push, scan_block_comment, scan_line_comment, scan_number, scan_string,
};

const KEYWORDS: &[&str] = &[
    "fn", "let", "mut", "pub", "struct", "enum", "impl", "use", "crate", "mod", "async", "await",
    "self", "Self", "where", "trait", "type",
];
const CONTROLS: &[&str] = &[
    "if", "else", "match", "return", "loop", "while", "for", "break", "continue",
];
const TYPES: &[&str] = &[
    "i32", "i64", "u32", "u64", "usize", "isize", "bool", "char", "str", "String", "Vec", "Option",
    "Result",
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
            c if is_ident_start(c) => crate::lexers::scan_ident_classified(
                &chars,
                &mut i,
                &mut tokens,
                KEYWORDS,
                CONTROLS,
                TYPES,
            ),
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
