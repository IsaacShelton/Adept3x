use num_bigint::BigInt;

use crate::{line_column::Location, token::{Token, TokenInfo}};

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

    pub fn to_token_info(&self) -> TokenInfo {
        match self.value.parse::<BigInt>() {
            Ok(value) => return TokenInfo::new(Token::Integer { value }, self.start_location),
            _ => (),
        }

        match self.value.parse::<f64>() {
            Ok(value) => return TokenInfo::new(Token::Float { value }, self.start_location),
            _ => (),
        }

        TokenInfo::new(Token::Error("Invalid number".into()), self.start_location)
    }
}
