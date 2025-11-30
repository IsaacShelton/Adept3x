use infinite_iterator::InfinitePeekable;
use std::collections::HashMap;
use token::{StringModifier, Token, TokenEat, TokenKind};

#[derive(Clone, Debug)]
pub enum Aon {
    Null,
    Integer(i64),
    String(String),
    Array(Vec<Aon>),
    Object(HashMap<String, Aon>),
}

impl Aon {
    pub fn get(&self, key: &str) -> Option<&Aon> {
        match self {
            Aon::Object(map) => map.get(key),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Aon::String(string) => Some(string),
            _ => None,
        }
    }
}

pub fn parse_aon<S: Copy, I: InfinitePeekable<Token<S>>>(lexer: &mut I) -> Result<Aon, ()> {
    let result = parse_aon_inner(lexer, 10)?;
    if !lexer.peek().is_end_of_file() {
        return Err(());
    }
    Ok(result)
}

fn parse_aon_inner<S: Copy, I: InfinitePeekable<Token<S>>>(
    lexer: &mut I,
    max_depth_left: usize,
) -> Result<Aon, ()> {
    if max_depth_left == 0 {
        return Err(());
    }
    let max_depth_left = max_depth_left - 1;
    lexer.eat_newlines();

    if let Some(value) = lexer.eat_integer() {
        lexer.eat_newlines();
        return Ok(Aon::Integer(i64::try_from(value).map_err(|_| ())?));
    }

    if let Some(string) = lexer.eat_string() {
        let StringModifier::Normal = string.modifier else {
            return Err(());
        };

        lexer.eat_newlines();
        return Ok(Aon::String(string.value));
    }

    if lexer.eat(TokenKind::NullKeyword) {
        lexer.eat_newlines();
        return Ok(Aon::Null);
    }

    if lexer.eat(TokenKind::OpenBracket) {
        lexer.eat_newlines();

        let mut items = vec![];
        let mut has_comma = true;

        while has_comma && !lexer.peek().is_close_bracket() && !lexer.peek().is_end_of_file() {
            items.push(parse_aon_inner(lexer, max_depth_left)?);
            lexer.eat_newlines();
            has_comma = lexer.eat(TokenKind::Comma);
            lexer.eat_newlines();
        }

        if !lexer.eat(TokenKind::CloseBracket) {
            return Err(());
        }

        lexer.eat_newlines();
        return Ok(Aon::Array(items));
    }

    if lexer.eat(TokenKind::OpenCurly) {
        lexer.eat_newlines();

        let mut map = HashMap::new();
        let mut has_comma = true;

        while has_comma && !lexer.peek().is_close_curly() && !lexer.peek().is_end_of_file() {
            let Some(key) = lexer.eat_identifier() else {
                return Err(());
            };

            lexer.eat_newlines();

            if !lexer.eat(TokenKind::Colon) {
                return Err(());
            }

            lexer.eat_newlines();
            map.insert(key, parse_aon_inner(lexer, max_depth_left)?);
            lexer.eat_newlines();
            has_comma = lexer.eat(TokenKind::Comma);
            lexer.eat_newlines();
        }

        if !lexer.eat(TokenKind::CloseCurly) {
            return Err(());
        }

        lexer.eat_newlines();
        return Ok(Aon::Object(map));
    }

    return Err(());
}
