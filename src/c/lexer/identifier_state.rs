use crate::{
    c::token::{CToken, CTokenKind},
    line_column::Location,
};

pub struct IdentifierState {
    pub identifier: String,
    pub start_location: Location,
}

impl IdentifierState {
    pub fn to_token(&mut self) -> CToken {
        let identifier = std::mem::replace(&mut self.identifier, String::default());

        CToken::new(
            match identifier.as_str() {
                "auto" => CTokenKind::AutoKeyword,
                "break" => CTokenKind::BreakKeyword,
                "case" => CTokenKind::CaseKeyword,
                "char" => CTokenKind::CharKeyword,
                "const" => CTokenKind::ConstKeyword,
                "continue" => CTokenKind::ContinueKeyword,
                "default" => CTokenKind::DefaultKeyword,
                "do" => CTokenKind::DoKeyword,
                "double" => CTokenKind::DoubleKeyword,
                "else" => CTokenKind::ElseKeyword,
                "enum" => CTokenKind::EnumKeyword,
                "extern" => CTokenKind::ExternKeyword,
                "float" => CTokenKind::FloatKeyword,
                "for" => CTokenKind::ForKeyword,
                "goto" => CTokenKind::GotoKeyword,
                "if" => CTokenKind::IfKeyword,
                "int" => CTokenKind::IntKeyword,
                "long" => CTokenKind::LongKeyword,
                "register" => CTokenKind::RegisterKeyword,
                "return" => CTokenKind::ReturnKeyword,
                "short" => CTokenKind::ShortKeyword,
                "signed" => CTokenKind::SignedKeyword,
                "sizeof" => CTokenKind::SizeofKeyword,
                "static" => CTokenKind::StaticKeyword,
                "struct" => CTokenKind::StructKeyword,
                "switch" => CTokenKind::SwitchKeyword,
                "typedef" => CTokenKind::TypedefKeyword,
                "union" => CTokenKind::UnionKeyword,
                "unsigned" => CTokenKind::UnsignedKeyword,
                "void" => CTokenKind::VoidKeyword,
                "volatile" => CTokenKind::VolatileKeyword,
                "while" => CTokenKind::WhileKeyword,
                _ => CTokenKind::Identifier(identifier),
            },
            self.start_location,
        )
    }
}
