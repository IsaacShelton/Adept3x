use super::number::lex_number;
use c_token::{CToken, CTokenKind, Invalid};
use infinite_iterator::{InfiniteIterator, InfinitePeekable};
use pp_token::{PreToken, PreTokenKind};
pub struct Lexer<I: InfinitePeekable<PreToken>> {
    pub input: I,
}

impl<I> Lexer<I>
where
    I: InfinitePeekable<PreToken>,
{
    pub fn new(input: I) -> Self {
        Self { input }
    }
}

impl<I> InfiniteIterator for Lexer<I>
where
    I: InfinitePeekable<PreToken>,
{
    type Item = CToken;

    fn next(&mut self) -> Self::Item {
        let PreToken { kind, source } = self.input.next();

        let kind = match kind {
            PreTokenKind::EndOfSequence => return CTokenKind::EndOfFile.at(source),
            PreTokenKind::Identifier(name) | PreTokenKind::ProtectedIdentifier(name) => {
                match name.as_str() {
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
                }
            }
            PreTokenKind::Number(number) => lex_number(number).unwrap_or_else(CTokenKind::Invalid),
            PreTokenKind::CharacterConstant(encoding, content) => {
                CTokenKind::CharacterConstant(encoding, content)
            }
            PreTokenKind::StringLiteral(encoding, content) => {
                CTokenKind::StringLiteral(encoding, content)
            }
            PreTokenKind::Punctuator(punctuator) => CTokenKind::Punctuator(punctuator),
            PreTokenKind::Other(..) => CTokenKind::Invalid(Invalid::UnrecognizedSymbol),
            PreTokenKind::UniversalCharacterName(..) => {
                CTokenKind::Invalid(Invalid::UniversalCharacterNameNotSupported)
            }
            PreTokenKind::HeaderName(_)
            | PreTokenKind::IsDefined(_)
            | PreTokenKind::Placeholder => {
                unreachable!("preprocessor byproducts still remain in output")
            }
        };

        CToken::new(kind, source)
    }
}
