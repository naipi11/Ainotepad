use crate::highlight::{Token, TokenKind};
use crate::lexers::{
    is_ident_start, push, scan_block_comment, scan_line_comment, scan_number, scan_string,
};

const KEYWORDS: &[&str] = &[
    "function",
    "let",
    "const",
    "var",
    "class",
    "import",
    "export",
    "async",
    "await",
    "type",
    "interface",
    "new",
    "this",
    "typeof",
    "instanceof",
];
const CONTROLS: &[&str] = &[
    "if", "else", "for", "while", "return", "break", "continue", "switch", "case", "try", "catch",
    "finally",
];
const TYPES: &[&str] = &[
    "string", "number", "boolean", "any", "void", "never", "Promise", "Array",
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
            crate::lexers::scan_ident_classified(
                &chars,
                &mut i,
                &mut tokens,
                KEYWORDS,
                CONTROLS,
                TYPES,
            );
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
