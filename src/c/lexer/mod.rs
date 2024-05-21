use super::{
    preprocessor::{PreToken, PreTokenKind},
    token::{CToken, CTokenKind, Integer},
};
use crate::{c::token::IntegerSuffix, line_column::Location};

#[derive(Clone)]
pub struct Lexer<'a, I: Iterator<Item = &'a PreToken> + Clone> {
    pub input: I,
}

impl<'a, I: Iterator<Item = &'a PreToken> + Clone> Lexer<'a, I> {
    pub fn new(input: I) -> Self {
        Self { input }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum LexError {
    UniversalCharacterNameNotSupported,
    UnrecognizedSymbol,
    UnrepresentableInteger,
}

impl<'a, I: Iterator<Item = &'a PreToken> + Clone> Iterator for Lexer<'a, I> {
    type Item = CToken;

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
                _ => CTokenKind::Identifier(name.to_string()),
            },
            PreTokenKind::Number(number) => match lex_number(&number) {
                Ok(token) => token,
                Err(err) => CTokenKind::LexError(err),
            },
            PreTokenKind::CharacterConstant(encoding, content) => {
                CTokenKind::CharacterConstant(encoding.clone(), content.clone())
            }
            PreTokenKind::StringLiteral(encoding, content) => {
                CTokenKind::StringLiteral(encoding.clone(), content.clone())
            }
            PreTokenKind::Punctuator(punctuator) => CTokenKind::Punctuator(punctuator.clone()),
            PreTokenKind::UniversalCharacterName(..) => {
                CTokenKind::LexError(LexError::UniversalCharacterNameNotSupported)
            }
            PreTokenKind::Other(..) => CTokenKind::LexError(LexError::UnrecognizedSymbol),
            PreTokenKind::HeaderName(_)
            | PreTokenKind::IsDefined(_)
            | PreTokenKind::Placeholder => unreachable!(),
        };

        Some(CToken::new(
            kind,
            Location::new(line.map(u32::from).unwrap_or(0), 1),
        ))
    }
}

fn lex_number(number: &str) -> Result<CTokenKind, LexError> {
    // TODO: Cleanup this procedure

    let number = number.replace("'", "");

    let (number, radix) = if number.starts_with("0") {
        (&number[..], 8)
    } else if number.starts_with("0x") || number.starts_with("0X") {
        (&number[2..], 16)
    } else if number.starts_with("0b") || number.starts_with("0B") {
        (&number[2..], 2)
    } else {
        (&number[..], 10)
    };

    // This part is ugly, but at least it's fast

    let (number, is_unsigned) = if number.ends_with("U") || number.ends_with("u") {
        (&number[..number.len() - 1], true)
    } else {
        (number, false)
    };

    let (number, is_long_long) = if number.ends_with("LL") || number.ends_with("ll") {
        (&number[..number.len() - 2], true)
    } else {
        (number, false)
    };

    let (number, is_long) = if number.ends_with("L") || number.ends_with("l") {
        (&number[..number.len() - 1], true)
    } else {
        (number, false)
    };

    let (number, is_unsigned) = if !is_unsigned && (number.ends_with("U") || number.ends_with("u"))
    {
        (&number[..number.len() - 1], true)
    } else {
        (number, is_unsigned)
    };

    let requested = match (is_unsigned, is_long, is_long_long) {
        (false, false, false) => IntegerSuffix::Int,
        (true, false, false) => IntegerSuffix::UnsignedInt,
        (false, true, false) => IntegerSuffix::Long,
        (true, true, false) => IntegerSuffix::UnsignedLong,
        (false, false, true) => IntegerSuffix::LongLong,
        (true, false, true) => IntegerSuffix::UnsignedLongLong,
        _ => unreachable!(),
    };

    // The correct type for an integer literal is whichever of these fits it first
    // (Section 6.4.4.1 of the C standard)
    let possibilities = match radix {
        10 => match requested {
            IntegerSuffix::Int => vec![
                IntegerSuffix::Int,
                IntegerSuffix::Long,
                IntegerSuffix::LongLong,
            ],
            IntegerSuffix::UnsignedInt => vec![
                IntegerSuffix::UnsignedInt,
                IntegerSuffix::UnsignedLong,
                IntegerSuffix::UnsignedLongLong,
            ],
            IntegerSuffix::Long => vec![IntegerSuffix::Long, IntegerSuffix::LongLong],
            IntegerSuffix::UnsignedLong => {
                vec![IntegerSuffix::UnsignedLong, IntegerSuffix::UnsignedLongLong]
            }
            IntegerSuffix::LongLong => vec![IntegerSuffix::LongLong],
            IntegerSuffix::UnsignedLongLong => vec![IntegerSuffix::UnsignedLongLong],
        },
        _ => match requested {
            IntegerSuffix::Int => vec![
                IntegerSuffix::Int,
                IntegerSuffix::UnsignedInt,
                IntegerSuffix::Long,
                IntegerSuffix::UnsignedLong,
                IntegerSuffix::LongLong,
                IntegerSuffix::UnsignedLongLong,
            ],
            IntegerSuffix::UnsignedInt => vec![
                IntegerSuffix::UnsignedInt,
                IntegerSuffix::UnsignedLong,
                IntegerSuffix::UnsignedLongLong,
            ],
            IntegerSuffix::Long => vec![
                IntegerSuffix::Long,
                IntegerSuffix::UnsignedLong,
                IntegerSuffix::LongLong,
                IntegerSuffix::UnsignedLongLong,
            ],
            IntegerSuffix::UnsignedLong => {
                vec![IntegerSuffix::UnsignedLong, IntegerSuffix::UnsignedLongLong]
            }
            IntegerSuffix::LongLong => {
                vec![IntegerSuffix::LongLong, IntegerSuffix::UnsignedLongLong]
            }
            IntegerSuffix::UnsignedLongLong => vec![IntegerSuffix::UnsignedLongLong],
        },
    };

    for possible_type in possibilities {
        if let Some(integer) = Integer::try_new(number, possible_type, radix) {
            return Ok(CTokenKind::Integer(integer));
        }
    }

    Err(LexError::UnrepresentableInteger)
}
