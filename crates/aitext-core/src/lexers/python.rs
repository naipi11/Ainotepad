use crate::highlight::{Token, TokenKind};
use crate::lexers::{
    is_ident_start, push, scan_ident_or_keyword, scan_line_comment, scan_number,
};

const KEYWORDS: &[&str] = &[
    "def", "class", "return", "if", "else", "elif", "for", "while", "import", "from", "as", "try",
    "except", "with", "pass", "True", "False", "None",
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
            c if is_ident_start(c) => scan_ident_or_keyword(&chars, &mut i, &mut tokens, KEYWORDS),
            '(' | ')' | '[' | ']' | '{' | '}' | ':' | ',' | '.' | '=' => {
                push(&mut tokens, i, i + 1, TokenKind::Punct);
                i += 1;
            }
            _ => i += 1,
        }
    }
    tokens
}
