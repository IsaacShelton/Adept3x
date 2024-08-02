use crate::{
    source_files::Source,
    token::{Token, TokenKind},
};
use num_bigint::BigInt;
use num_traits::Num;

pub struct HexNumberState {
    pub value: String,
    pub start_source: Source,
}

impl HexNumberState {
    pub fn to_token(&self) -> Token {
        match BigInt::from_str_radix(&self.value, 16) {
            Ok(value) => TokenKind::Integer(value),
            Err(_) => TokenKind::Error(format!("Invalid hex number {}", &self.value)),
        }
        .at(self.start_source)
    }
}
