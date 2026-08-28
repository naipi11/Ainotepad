use crate::highlight::{Token, TokenKind};
use crate::lexers::{is_ident_continue, is_ident_start, push, scan_string};

pub fn lex(text: &str) -> Vec<Token> {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut tokens = Vec::new();
    while i < chars.len() {
        if i + 3 < chars.len()
            && chars[i] == '<'
            && chars[i + 1] == '!'
            && chars[i + 2] == '-'
            && chars[i + 3] == '-'
        {
            let start = i;
            i += 4;
            while i + 2 < chars.len()
                && !(chars[i] == '-' && chars[i + 1] == '-' && chars[i + 2] == '>')
            {
                i += 1;
            }
            i = (i + 3).min(chars.len());
            push(&mut tokens, start, i, TokenKind::Comment);
            continue;
        }
        if chars[i] == '<' {
            let start = i;
            i += 1;
            if i < chars.len() && chars[i] == '/' {
                i += 1;
            }
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            let name_start = i;
            if i < chars.len() && (is_ident_start(chars[i]) || chars[i] == '!') {
                i += 1;
                while i < chars.len() && is_ident_continue(chars[i])
                    || i < chars.len() && chars[i] == '-'
                {
                    i += 1;
                }
            }
            if i > name_start {
                push(&mut tokens, start, i, TokenKind::Keyword);
            } else {
                push(&mut tokens, start, i, TokenKind::Punct);
            }
            while i < chars.len() && chars[i] != '>' {
                if chars[i] == '"' || chars[i] == '\'' {
                    let quote = chars[i];
                    scan_string(&chars, &mut i, &mut tokens, quote);
                } else if is_ident_start(chars[i]) {
                    let attr_start = i;
                    i += 1;
                    while i < chars.len()
                        && (is_ident_continue(chars[i]) || chars[i] == '-' || chars[i] == ':')
                    {
                        i += 1;
                    }
                    push(&mut tokens, attr_start, i, TokenKind::Ident);
                } else {
                    i += 1;
                }
            }
            if i < chars.len() {
                push(&mut tokens, i, i + 1, TokenKind::Punct);
                i += 1;
            }
            continue;
        }
        i += 1;
    }
    tokens
}
