use crate::{
    source_files::Source,
    token::{Token, TokenKind},
};

pub struct IdentifierState {
    pub identifier: String,
    pub start_source: Source,
}

impl IdentifierState {
    pub fn finalize(&mut self) -> Token {
        let identifier = std::mem::take(&mut self.identifier);

        match identifier.as_str() {
            "func" => TokenKind::FuncKeyword,
            "return" => TokenKind::ReturnKeyword,
            "struct" => TokenKind::StructKeyword,
            "union" => TokenKind::UnionKeyword,
            "enum" => TokenKind::EnumKeyword,
            "typealias" => TokenKind::TypeAliasKeyword,
            "if" => TokenKind::IfKeyword,
            "else" => TokenKind::ElseKeyword,
            "elif" => TokenKind::ElifKeyword,
            "while" => TokenKind::WhileKeyword,
            "true" => TokenKind::TrueKeyword,
            "false" => TokenKind::FalseKeyword,
            "define" => TokenKind::DefineKeyword,
            "zeroed" => TokenKind::ZeroedKeyword,
            "pragma" => TokenKind::PragmaKeyword,
            _ => TokenKind::Identifier(identifier),
        }
        .at(self.start_source)
    }
}
