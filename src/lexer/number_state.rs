use crate::{
    line_column::Location,
    token::{TokenKind, Token},
};
use num_bigint::BigInt;

pub struct NumberState {
    pub value: String,
    pub can_dot: bool,
    pub can_exp: bool,
    pub can_neg: bool,
    pub start_location: Location,
}

impl NumberState {
    pub fn new(value: String, start_location: Location) -> Self {
        Self {
            value,
            can_dot: true,
            can_exp: true,
            can_neg: false,
            start_location,
        }
    }

    pub fn to_token(&self) -> Token {
        match self.value.parse::<BigInt>() {
            Ok(value) => return Token::new(TokenKind::Integer { value }, self.start_location),
            _ => (),
        }

        match self.value.parse::<f64>() {
            Ok(value) => return Token::new(TokenKind::Float { value }, self.start_location),
            _ => (),
        }

        Token::new(TokenKind::Error("Invalid number".into()), self.start_location)
    }
}
