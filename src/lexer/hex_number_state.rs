use num_bigint::BigInt;
use num_traits::Num;

use crate::{
    line_column::Location,
    token::{TokenKind, Token},
};

pub struct HexNumberState {
    pub value: String,
    pub start_location: Location,
}

impl HexNumberState {
    pub fn to_token(&self) -> Token {
        Token::new(
            match BigInt::from_str_radix(&self.value, 16) {
                Ok(value) => TokenKind::Integer(value),
                Err(_) => TokenKind::Error(format!("Invalid hex number {}", &self.value)),
            },
            self.start_location,
        )
    }
}
