use super::{
    preprocessor::{PreToken, PreTokenKind},
    token::{CToken, CTokenKind},
};
use crate::line_column::Location;

pub struct Lexer<I: Iterator<Item = PreToken>> {
    pub input: I,
}

#[derive(Clone, Debug)]
pub enum LexError {
    UniversalCharacterNameNotSupported,
    UnrecognizedSymbol,
}

impl<I: Iterator<Item = PreToken>> Iterator for Lexer<I> {
    type Item = Result<CToken, LexError>;

    fn next(&mut self) -> Option<Self::Item> {
        let PreToken { kind, line } = self.input.next()?;

        let kind = match kind {
            PreTokenKind::Identifier(name) => match name.as_str() {
                "alignas" | "_Alignas" => CTokenKind::AlignasKeyword,
                "alignof" | "_Alignof" => CTokenKind::AlignofKeyword,
                "auto" => CTokenKind::AutoKeyword,
                "bool" | "_Bool" => CTokenKind::BoolKeyword,
                "break" => CTokenKind::BreakKeyword,
                "case" => CTokenKind::CaseKeyword,
                "char" => CTokenKind::CharKeyword,
                "const" => CTokenKind::ConstKeyword,
                "constexpr" => CTokenKind::ConstexprKeyword,
                "continue" => CTokenKind::ContinueKeyword,
                "default" => CTokenKind::DefaultKeyword,
                "do" => CTokenKind::DoKeyword,
                "double" => CTokenKind::DoubleKeyword,
                "else" => CTokenKind::ElseKeyword,
                "enum" => CTokenKind::EnumKeyword,
                "extern" => CTokenKind::ExternKeyword,
                "false" => CTokenKind::FalseKeyword,
                "float" => CTokenKind::FloatKeyword,
                "for" => CTokenKind::ForKeyword,
                "goto" => CTokenKind::GotoKeyword,
                "if" => CTokenKind::IfKeyword,
                "inline" => CTokenKind::InlineKeyword,
                "int" => CTokenKind::IntKeyword,
                "long" => CTokenKind::LongKeyword,
                "nullptr" => CTokenKind::NullptrKeyword,
                "register" => CTokenKind::RegisterKeyword,
                "restrict" => CTokenKind::RestrictKeyword,
                "return" => CTokenKind::ReturnKeyword,
                "short" => CTokenKind::ShortKeyword,
                "signed" => CTokenKind::SignedKeyword,
                "sizeof" => CTokenKind::SizeofKeyword,
                "static" => CTokenKind::StaticKeyword,
                "static_assert" | "_Static_assert" => CTokenKind::StaticAssertKeyword,
                "struct" => CTokenKind::StructKeyword,
                "switch" => CTokenKind::SwitchKeyword,
                "thread_local" | "_Thread_local" => CTokenKind::ThreadLocalKeyword,
                "true" => CTokenKind::TrueKeyword,
                "typedef" => CTokenKind::TypedefKeyword,
                "typeof" => CTokenKind::TypeofKeyword,
                "typeof_unqual" => CTokenKind::TypeofUnqualKeyword,
                "union" => CTokenKind::UnionKeyword,
                "unsigned" => CTokenKind::UnsignedKeyword,
                "void" => CTokenKind::VoidKeyword,
                "volatile" => CTokenKind::VolatileKeyword,
                "while" => CTokenKind::WhileKeyword,
                "_Atomic" => CTokenKind::AtomicKeyword,
                "_BitInt" => CTokenKind::BitIntKeyword,
                "_Complex" => CTokenKind::ComplexKeyword,
                "_Decimal128" => CTokenKind::Decimal128Keyword,
                "_Decimal32" => CTokenKind::Decimal32Keyword,
                "_Decimal64" => CTokenKind::Decimal64Keyword,
                "_Generic" => CTokenKind::GenericKeyword,
                "_Imaginary" => CTokenKind::ImaginaryKeyword,
                "_Noreturn" => CTokenKind::NoreturnKeyword,
                _ => CTokenKind::Identifier(name),
            },
            PreTokenKind::Number(number) => match lex_number(&number) {
                Ok(token) => token,
                Err(err) => return Some(Err(err)),
            },
            PreTokenKind::CharacterConstant(encoding, content) => {
                CTokenKind::CharacterConstant(encoding, content)
            }
            PreTokenKind::StringLiteral(encoding, content) => {
                CTokenKind::StringLiteral(encoding, content)
            }
            PreTokenKind::Punctuator(punctuator) => CTokenKind::Punctuator(punctuator),
            PreTokenKind::UniversalCharacterName(..) => {
                return Some(Err(LexError::UniversalCharacterNameNotSupported));
            }
            PreTokenKind::Other(..) => {
                return Some(Err(LexError::UnrecognizedSymbol));
            }
            PreTokenKind::HeaderName(_)
            | PreTokenKind::IsDefined(_)
            | PreTokenKind::Placeholder => unreachable!(),
        };

        Some(Ok(CToken::new(
            kind,
            Location::new(line.map(u32::from).unwrap_or(0), 1),
        )))
    }
}

fn lex_number(_number: &str) -> Result<CTokenKind, LexError> {
    unimplemented!("lexing C numbers is not implemented yet");
}
