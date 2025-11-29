use num_bigint::BigInt;
use token::{Token, TokenKind};

pub struct NumberState<S: Copy> {
    pub value: String,
    pub can_dot: bool,
    pub can_exp: bool,
    pub can_neg: bool,
    pub start_source: S,
}

impl<S: Copy> NumberState<S> {
    pub fn new(value: String, start_source: S) -> Self {
        Self {
            value,
            can_dot: true,
            can_exp: true,
            can_neg: false,
            start_source,
        }
    }

    pub fn to_token(&self) -> Token<S> {
        if let Ok(value) = self.value.parse::<BigInt>() {
            return TokenKind::Integer(value).at(self.start_source);
        }

        if let Ok(value) = self.value.parse::<f64>() {
            return TokenKind::Float(value).at(self.start_source);
        }

        TokenKind::Error("Invalid number".into()).at(self.start_source)
    }
}
