use crate::highlight::{Token, TokenKind};
use crate::lexers::{
    is_ident_start, push, scan_block_comment, scan_ident_classified, scan_line_comment,
    scan_number, scan_string,
};

const KEYWORDS: &[&str] = &[
    "using",
    "namespace",
    "class",
    "struct",
    "interface",
    "enum",
    "public",
    "private",
    "protected",
    "internal",
    "static",
    "readonly",
    "const",
    "new",
    "async",
    "await",
    "var",
    "this",
    "base",
    "throw",
];

const CONTROLS: &[&str] = &[
    "if", "else", "for", "foreach", "while", "do", "switch", "case", "break", "continue", "return",
    "try", "catch", "finally",
];

const TYPES: &[&str] = &[
    "void", "bool", "byte", "char", "decimal", "double", "float", "int", "long", "object", "sbyte",
    "short", "string", "uint", "ulong", "ushort", "dynamic", "Task",
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
            '"' | '\'' => {
                let quote = chars[i];
                scan_string(&chars, &mut i, &mut tokens, quote);
            }
            c if c.is_ascii_digit() => scan_number(&chars, &mut i, &mut tokens),
            c if is_ident_start(c) => {
                scan_ident_classified(&chars, &mut i, &mut tokens, KEYWORDS, CONTROLS, TYPES)
            }
            '{' | '}' | '(' | ')' | '[' | ']' | ';' | ',' | '.' | ':' => {
                push(&mut tokens, i, i + 1, TokenKind::Punct);
                i += 1;
            }
            _ => i += 1,
        }
    }
    tokens
}
