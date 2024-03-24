use crate::{
    line_column::Location,
    token::{TokenKind, Token},
};

pub struct IdentifierState {
    pub identifier: String,
    pub start_location: Location,
}

impl IdentifierState {
    pub fn to_token(&mut self) -> Token {
        let identifier = std::mem::replace(&mut self.identifier, String::default());

        Token::new(
            match identifier.as_str() {
                "func" => TokenKind::FuncKeyword,
                "return" => TokenKind::ReturnKeyword,
                "struct" => TokenKind::StructKeyword,
                "if" => TokenKind::IfKeyword,
                "else" => TokenKind::ElseKeyword,
                "elif" => TokenKind::ElifKeyword,
                _ => TokenKind::Identifier(identifier),
            },
            self.start_location,
        )
    }
}
