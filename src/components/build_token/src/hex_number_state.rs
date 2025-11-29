use num_bigint::BigInt;
use num_traits::Num;
use token::{Token, TokenKind};

pub struct HexNumberState<S: Copy> {
    pub value: String,
    pub start_source: S,
}

impl<S: Copy> HexNumberState<S> {
    pub fn to_token(&self) -> Token<S> {
        match BigInt::from_str_radix(&self.value, 16) {
            Ok(value) => TokenKind::Integer(value),
            Err(_) => TokenKind::Error(format!("Invalid hex number {}", &self.value)),
        }
        .at(self.start_source)
    }
}
