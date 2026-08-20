use crate::highlight::{Token, TokenKind};
use crate::lexers::{is_ident_start, push, scan_line_comment, scan_number};

const KEYWORDS: &[&str] = &[
    "def", "class", "lambda", "import", "from", "as", "with", "pass", "global", "nonlocal",
    "assert", "yield", "del", "raise", "True", "False", "None", "and", "or", "not", "in", "is",
];
const CONTROLS: &[&str] = &[
    "if", "else", "elif", "for", "while", "try", "except", "finally", "return", "break", "continue",
];
const TYPES: &[&str] = &[
    "int", "str", "float", "bool", "list", "dict", "set", "tuple", "bytes", "object",
];

pub fn lex(text: &str) -> Vec<Token> {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut tokens = Vec::new();
    while i < chars.len() {
        if chars[i] == '#' {
            scan_line_comment(&chars, &mut i, &mut tokens, 1);
            continue;
        }
        if i + 2 < chars.len()
            && ((chars[i] == '"' && chars[i + 1] == '"' && chars[i + 2] == '"')
                || (chars[i] == '\'' && chars[i + 1] == '\'' && chars[i + 2] == '\''))
        {
            let quote = chars[i];
            let start = i;
            i += 3;
            while i + 2 < chars.len()
                && !(chars[i] == quote && chars[i + 1] == quote && chars[i + 2] == quote)
            {
                i += 1;
            }
            i = (i + 3).min(chars.len());
            push(&mut tokens, start, i, TokenKind::String);
            continue;
        }
        match chars[i] {
            '"' | '\'' => {
                let q = chars[i];
                crate::lexers::scan_string(&chars, &mut i, &mut tokens, q);
            }
            c if c.is_ascii_digit() => scan_number(&chars, &mut i, &mut tokens),
            c if is_ident_start(c) => crate::lexers::scan_ident_classified(
                &chars,
                &mut i,
                &mut tokens,
                KEYWORDS,
                CONTROLS,
                TYPES,
            ),
            '(' | ')' | '[' | ']' | '{' | '}' | ':' | ',' | '.' | '=' => {
                push(&mut tokens, i, i + 1, TokenKind::Punct);
                i += 1;
            }
            _ => i += 1,
        }
    }
    tokens
}
