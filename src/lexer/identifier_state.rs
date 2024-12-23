use crate::{
    name::Name,
    source_files::Source,
    token::{Token, TokenKind},
};

pub struct IdentifierState {
    pub identifier: String,
    pub start_source: Source,
    pub last_slash: Option<usize>,
}

impl IdentifierState {
    pub fn finalize(&mut self) -> Token {
        let mut identifier = std::mem::take(&mut self.identifier);

        if let Some(last_slash) = self.last_slash {
            let basename = identifier.split_off(last_slash + 1);

            // Remove trailing slash
            identifier.pop();

            let namespace = identifier;

            return TokenKind::NamespacedIdentifier(Name::new(Some(namespace), basename))
                .at(self.start_source);
        }

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
            "pub" => TokenKind::PubKeyword,
            "trait" => TokenKind::TraitKeyword,
            "impl" => TokenKind::ImplKeyword,
            "for" => TokenKind::ForKeyword,
            "is" => TokenKind::IsKeyword,
            "given" => TokenKind::GivenKeyword,
            _ => TokenKind::Identifier(identifier),
        }
        .at(self.start_source)
    }
}
