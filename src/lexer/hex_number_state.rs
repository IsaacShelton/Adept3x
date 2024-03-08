use num_bigint::BigInt;
use num_traits::Num;

use crate::{
    line_column::Location,
    token::{Token, TokenInfo},
};

pub struct HexNumberState {
    pub value: String,
    pub start_location: Location,
}

impl HexNumberState {
    pub fn to_token_info(&self) -> TokenInfo {
        TokenInfo::new(
            match BigInt::from_str_radix(&self.value, 16) {
                Ok(value) => Token::Integer { value },
                Err(_) => Token::Error(format!("Invalid hex number {}", &self.value)),
            },
            self.start_location,
        )
    }
}
