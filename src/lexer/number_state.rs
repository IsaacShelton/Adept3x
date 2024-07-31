use crate::{
    ast::Source,
    token::{Token, TokenKind},
};
use num_bigint::BigInt;

pub struct NumberState {
    pub value: String,
    pub can_dot: bool,
    pub can_exp: bool,
    pub can_neg: bool,
    pub start_source: Source,
}

impl NumberState {
    pub fn new(value: String, start_source: Source) -> Self {
        Self {
            value,
            can_dot: true,
            can_exp: true,
            can_neg: false,
            start_source,
        }
    }

    pub fn to_token(&self) -> Token {
        if let Ok(value) = self.value.parse::<BigInt>() {
            return TokenKind::Integer(value).at(self.start_source);
        }

        if let Ok(value) = self.value.parse::<f64>() {
            return TokenKind::Float(value).at(self.start_source);
        }

        TokenKind::Error("Invalid number".into()).at(self.start_source)
    }
}
