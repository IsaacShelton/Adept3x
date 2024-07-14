use crate::{
    line_column::Location,
    token::{Token, TokenKind},
};

pub struct IdentifierState {
    pub identifier: String,
    pub start_location: Location,
}

impl IdentifierState {
    pub fn finalize(&mut self) -> Token {
        let identifier = std::mem::take(&mut self.identifier);

        Token::new(
            match identifier.as_str() {
                "func" => TokenKind::FuncKeyword,
                "return" => TokenKind::ReturnKeyword,
                "struct" => TokenKind::StructKeyword,
                "union" => TokenKind::UnionKeyword,
                "enum" => TokenKind::EnumKeyword,
                "alias" => TokenKind::AliasKeyword,
                "if" => TokenKind::IfKeyword,
                "else" => TokenKind::ElseKeyword,
                "elif" => TokenKind::ElifKeyword,
                "while" => TokenKind::WhileKeyword,
                "true" => TokenKind::TrueKeyword,
                "false" => TokenKind::FalseKeyword,
                "define" => TokenKind::DefineKeyword,
                "zeroed" => TokenKind::ZeroedKeyword,
                _ => TokenKind::Identifier(identifier),
            },
            self.start_location,
        )
    }
}
