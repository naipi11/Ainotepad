use crate::highlight::{Token, TokenKind};
use crate::lexers::push;

pub fn lex(text: &str) -> Vec<Token> {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut tokens = Vec::new();
    while i < chars.len() {
        if chars[i] == '#' && (i == 0 || chars[i - 1] == '\n') {
            let start = i;
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            push(&mut tokens, start, i, TokenKind::Keyword);
            continue;
        }
        if i + 2 < chars.len() && chars[i] == '`' && chars[i + 1] == '`' && chars[i + 2] == '`' {
            let start = i;
            i += 3;
            while i + 2 < chars.len()
                && !(chars[i] == '`' && chars[i + 1] == '`' && chars[i + 2] == '`')
            {
                i += 1;
            }
            i = (i + 3).min(chars.len());
            push(&mut tokens, start, i, TokenKind::String);
            continue;
        }
        if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '*' {
            let start = i;
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '*') {
                i += 1;
            }
            i = (i + 2).min(chars.len());
            push(&mut tokens, start, i, TokenKind::Keyword);
            continue;
        }
        if chars[i] == '*' {
            let start = i;
            i += 1;
            while i < chars.len() && chars[i] != '*' && chars[i] != '\n' {
                i += 1;
            }
            if i < chars.len() && chars[i] == '*' {
                i += 1;
            }
            push(&mut tokens, start, i, TokenKind::Keyword);
            continue;
        }
        i += 1;
    }
    tokens
}
