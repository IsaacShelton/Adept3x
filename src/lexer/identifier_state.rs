use crate::{line_column::Location, token::{Token, TokenInfo}};

pub struct IdentifierState {
    pub identifier: String,
    pub start_location: Location,
}

impl IdentifierState {
    pub fn to_token_info(&mut self) -> TokenInfo {
        let identifier = std::mem::replace(&mut self.identifier, String::default());

        TokenInfo::new(
            match identifier.as_str() {
                "func" => Token::FuncKeyword,
                "return" => Token::ReturnKeyword,
                _ => Token::Identifier(identifier),
            },
            self.start_location,
        )
    }
}
