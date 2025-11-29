use token::{Token, TokenKind};

pub struct IdentifierState<S: Copy> {
    pub identifier: String,
    pub start_source: S,
    pub last_slash: Option<usize>,
}

impl<S: Copy> IdentifierState<S> {
    pub fn finalize(&mut self) -> Token<S> {
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
            "pub" => TokenKind::PubKeyword,
            "trait" => TokenKind::TraitKeyword,
            "impl" => TokenKind::ImplKeyword,
            "for" => TokenKind::ForKeyword,
            "is" => TokenKind::IsKeyword,
            "null" => TokenKind::NullKeyword,
            "break" => TokenKind::BreakKeyword,
            "continue" => TokenKind::ContinueKeyword,
            "namespace" => TokenKind::NamespaceKeyword,
            "mod" => TokenKind::ModKeyword,
            "goto" => TokenKind::GotoKeyword,
            "when" => TokenKind::WhenKeyword,
            "linkset" => TokenKind::LinksetKeyword,
            _ => TokenKind::Identifier(identifier),
        }
        .at(self.start_source)
    }
}
