use std::collections::HashMap;
use super::token::TokenKind;

pub fn is_keyword(word: &str) -> bool {
    KEYWORDS.contains_key(word)
}

pub fn get_keyword(word: &str) -> Option<TokenKind> {
    KEYWORDS.get(word).cloned()
}

lazy_static::lazy_static! {
    static ref KEYWORDS: HashMap<&'static str, TokenKind> = {
        let mut map = HashMap::new();
        map.insert("store", TokenKind::Store);
        map.insert("define", TokenKind::Define);
        map.insert("return", TokenKind::Return);
        map.insert("if", TokenKind::If);
        map.insert("else", TokenKind::Else);
        map.insert("elseif", TokenKind::Elseif);
        map.insert("end", TokenKind::End);
        map.insert("while", TokenKind::While);
        map.insert("for", TokenKind::For);
        map.insert("in", TokenKind::In);
        map.insert("repeat", TokenKind::Repeat);
        map.insert("times", TokenKind::Times);
        map.insert("match", TokenKind::Match);
        map.insert("case", TokenKind::Case);
        map.insert("break", TokenKind::Break);
        map.insert("continue", TokenKind::Continue);
        map.insert("try", TokenKind::Try);
        map.insert("catch", TokenKind::Catch);
        map.insert("import", TokenKind::Import);
        map.insert("from", TokenKind::From);
        map.insert("as", TokenKind::As);
        map.insert("and", TokenKind::And);
        map.insert("or", TokenKind::Or);
        map.insert("not", TokenKind::Not);
        map.insert("assert", TokenKind::Assert);
        map.insert("true", TokenKind::Boolean(true));
        map.insert("false", TokenKind::Boolean(false));
        map.insert("null", TokenKind::Null);
        map
    };
}

