use crate::highlight::{Token, TokenKind};

pub fn lex(text: &str) -> Vec<Token> {
    vec![Token {
        start: 0,
        end: text.chars().count(),
        kind: TokenKind::Text,
    }]
}
