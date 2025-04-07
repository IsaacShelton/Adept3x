use derive_more::{Deref, IsVariant, Unwrap};
use inflow::InflowEnd;
use num_bigint::BigInt;
pub use pp_token::{Encoding, Punctuator};
use source_files::Source;
use std::fmt::Display;

#[derive(Clone, Debug, PartialEq, IsVariant, Unwrap)]
pub enum CTokenKind {
    EndOfFile,
    Invalid(Invalid),
    Identifier(String),
    Punctuator(Punctuator),
    Integer(Integer),
    Float(f64, FloatSuffix),
    CharacterConstant(Encoding, String),
    StringLiteral(Encoding, String),
    AlignasKeyword,
    AlignofKeyword,
    AutoKeyword,
    BoolKeyword,
    BreakKeyword,
    CaseKeyword,
    CharKeyword,
    ConstKeyword,
    ConstexprKeyword,
    ContinueKeyword,
    DefaultKeyword,
    DoKeyword,
    DoubleKeyword,
    ElseKeyword,
    EnumKeyword,
    ExternKeyword,
    FalseKeyword,
    FloatKeyword,
    ForKeyword,
    GotoKeyword,
    IfKeyword,
    InlineKeyword,
    IntKeyword,
    LongKeyword,
    NullptrKeyword,
    RegisterKeyword,
    RestrictKeyword,
    ReturnKeyword,
    ShortKeyword,
    SignedKeyword,
    SizeofKeyword,
    StaticKeyword,
    StaticAssertKeyword,
    StructKeyword,
    SwitchKeyword,
    ThreadLocalKeyword,
    TrueKeyword,
    TypedefKeyword,
    TypeofKeyword,
    TypeofUnqualKeyword,
    UnionKeyword,
    UnsignedKeyword,
    VoidKeyword,
    VolatileKeyword,
    WhileKeyword,
    AtomicKeyword,
    BitIntKeyword,
    ComplexKeyword,
    Decimal128Keyword,
    Decimal32Keyword,
    Decimal64Keyword,
    GenericKeyword,
    ImaginaryKeyword,
    NoreturnKeyword,
}

impl CTokenKind {
    pub fn at(self, source: Source) -> CToken {
        CToken { kind: self, source }
    }

    pub fn is_open_paren(&self) -> bool {
        matches!(self, CTokenKind::Punctuator(Punctuator::OpenParen { .. }))
    }

    pub fn precedence(&self) -> usize {
        let punctuator = match self {
            Self::Punctuator(punctuator) => punctuator,
            _ => return 0,
        };

        match punctuator {
            Punctuator::Dot | Punctuator::Arrow => 15,
            Punctuator::Increment
            | Punctuator::Decrement
            | Punctuator::Not
            | Punctuator::BitComplement => 14,
            Punctuator::Multiply | Punctuator::Divide | Punctuator::Modulus => 12,
            Punctuator::Add | Punctuator::Subtract => 11,
            Punctuator::LeftShift | Punctuator::RightShift => 10,
            Punctuator::LessThan
            | Punctuator::GreaterThan
            | Punctuator::LessThanEq
            | Punctuator::GreaterThanEq => 9,
            Punctuator::DoubleEquals | Punctuator::NotEquals => 8,
            Punctuator::Ampersand => 7,
            Punctuator::BitXor => 6,
            Punctuator::BitOr => 5,
            Punctuator::LogicalAnd => 4,
            Punctuator::LogicalOr => 3,
            Punctuator::Ternary => 2,
            Punctuator::MultiplyAssign
            | Punctuator::DivideAssign
            | Punctuator::ModulusAssign
            | Punctuator::AddAssign
            | Punctuator::SubtractAssign
            | Punctuator::LeftShiftAssign
            | Punctuator::RightShiftAssign
            | Punctuator::BitAndAssign
            | Punctuator::BitXorAssign
            | Punctuator::BitOrAssign => 1,
            _ => 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Integer {
    Int(i32),
    UnsignedInt(u32),
    Long(i64),
    UnsignedLong(u64),
    LongLong(i64),
    UnsignedLongLong(u64),
}

impl From<&Integer> for BigInt {
    fn from(val: &Integer) -> Self {
        match val {
            Integer::Int(x) => BigInt::from(*x),
            Integer::UnsignedInt(x) => BigInt::from(*x),
            Integer::Long(x) => BigInt::from(*x),
            Integer::UnsignedLong(x) => BigInt::from(*x),
            Integer::LongLong(x) => BigInt::from(*x),
            Integer::UnsignedLongLong(x) => BigInt::from(*x),
        }
    }
}

impl Integer {
    pub fn try_new(representation: &str, suffix: IntegerSuffix, radix: u32) -> Option<Integer> {
        Some(match suffix {
            IntegerSuffix::Int => Integer::Int(i32::from_str_radix(representation, radix).ok()?),
            IntegerSuffix::UnsignedInt => {
                Integer::UnsignedInt(u32::from_str_radix(representation, radix).ok()?)
            }
            IntegerSuffix::Long => Integer::Long(i64::from_str_radix(representation, radix).ok()?),
            IntegerSuffix::UnsignedLong => {
                Integer::UnsignedLong(u64::from_str_radix(representation, radix).ok()?)
            }
            IntegerSuffix::LongLong => {
                Integer::LongLong(i64::from_str_radix(representation, radix).ok()?)
            }
            IntegerSuffix::UnsignedLongLong => {
                Integer::UnsignedLongLong(u64::from_str_radix(representation, radix).ok()?)
            }
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum IntegerSuffix {
    Int,
    UnsignedInt,
    Long,
    UnsignedLong,
    LongLong,
    UnsignedLongLong,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FloatSuffix {
    Float,
    LongDouble,
    Decimal32,
    Decimal64,
    Decimal128,
}

#[derive(Clone, Debug, Deref)]
pub struct CToken {
    #[deref]
    pub kind: CTokenKind,

    pub source: Source,
}

impl CToken {
    pub fn new(kind: CTokenKind, source: Source) -> CToken {
        CToken { kind, source }
    }
}

impl InflowEnd for CToken {
    fn is_inflow_end(&self) -> bool {
        matches!(&self.kind, CTokenKind::EndOfFile)
    }
}

impl Display for CTokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CTokenKind::EndOfFile => write!(f, "<end-of-file>"),
            CTokenKind::Invalid(lex_error) => lex_error.fmt(f),
            CTokenKind::Identifier(identifier) => write!(f, "identifier '{}'", identifier),
            CTokenKind::Punctuator(punctuator) => write!(f, "'{}'", punctuator),
            CTokenKind::Integer(_) => write!(f, "integer literal"),
            CTokenKind::Float(_, _) => write!(f, "floating-point literal"),
            CTokenKind::CharacterConstant(_, _) => write!(f, "character literal"),
            CTokenKind::StringLiteral(_, _) => write!(f, "string literal"),
            CTokenKind::AlignasKeyword => write!(f, "'alignas' keyword"),
            CTokenKind::AlignofKeyword => write!(f, "'alignof' keyword"),
            CTokenKind::AutoKeyword => write!(f, "'auto' keyword"),
            CTokenKind::BoolKeyword => write!(f, "'bool' keyword"),
            CTokenKind::BreakKeyword => write!(f, "'break' keyword"),
            CTokenKind::CaseKeyword => write!(f, "'case' keyword"),
            CTokenKind::CharKeyword => write!(f, "'char' keyword"),
            CTokenKind::ConstKeyword => write!(f, "'const' keyword"),
            CTokenKind::ConstexprKeyword => write!(f, "'constexpr' keyword"),
            CTokenKind::ContinueKeyword => write!(f, "'continue' keyword"),
            CTokenKind::DefaultKeyword => write!(f, "'default' keyword"),
            CTokenKind::DoKeyword => write!(f, "'do' keyword"),
            CTokenKind::DoubleKeyword => write!(f, "'double' keyword"),
            CTokenKind::ElseKeyword => write!(f, "'else' keyword"),
            CTokenKind::EnumKeyword => write!(f, "'enum' keyword"),
            CTokenKind::ExternKeyword => write!(f, "'extern' keyword"),
            CTokenKind::FalseKeyword => write!(f, "'false' keyword"),
            CTokenKind::FloatKeyword => write!(f, "'float' keyword"),
            CTokenKind::ForKeyword => write!(f, "'for' keyword"),
            CTokenKind::GotoKeyword => write!(f, "'goto' keyword"),
            CTokenKind::IfKeyword => write!(f, "'if' keyword"),
            CTokenKind::InlineKeyword => write!(f, "'inline' keyword"),
            CTokenKind::IntKeyword => write!(f, "'int' keyword"),
            CTokenKind::LongKeyword => write!(f, "'long' keyword"),
            CTokenKind::NullptrKeyword => write!(f, "'nullptr' keyword"),
            CTokenKind::RegisterKeyword => write!(f, "'register' keyword"),
            CTokenKind::RestrictKeyword => write!(f, "'restrict' keyword"),
            CTokenKind::ReturnKeyword => write!(f, "'return' keyword"),
            CTokenKind::ShortKeyword => write!(f, "'short' keyword"),
            CTokenKind::SignedKeyword => write!(f, "'signed' keyword"),
            CTokenKind::SizeofKeyword => write!(f, "'sizeof' keyword"),
            CTokenKind::StaticKeyword => write!(f, "'static' keyword"),
            CTokenKind::StaticAssertKeyword => write!(f, "'static_assert' keyword"),
            CTokenKind::StructKeyword => write!(f, "'struct' keyword"),
            CTokenKind::SwitchKeyword => write!(f, "'switch' keyword"),
            CTokenKind::ThreadLocalKeyword => write!(f, "'thread_local' keyword"),
            CTokenKind::TrueKeyword => write!(f, "'true' keyword"),
            CTokenKind::TypedefKeyword => write!(f, "'typedef' keyword"),
            CTokenKind::TypeofKeyword => write!(f, "'typeof' keyword"),
            CTokenKind::TypeofUnqualKeyword => write!(f, "'typeof_unqual' keyword"),
            CTokenKind::UnionKeyword => write!(f, "'union' keyword"),
            CTokenKind::UnsignedKeyword => write!(f, "'unsigned' keyword"),
            CTokenKind::VoidKeyword => write!(f, "'void' keyword"),
            CTokenKind::VolatileKeyword => write!(f, "'volatile' keyword"),
            CTokenKind::WhileKeyword => write!(f, "'while' keyword"),
            CTokenKind::AtomicKeyword => write!(f, "'_Atomic' keyword"),
            CTokenKind::BitIntKeyword => write!(f, "'_BitInt' keyword"),
            CTokenKind::ComplexKeyword => write!(f, "'_Complex' keyword"),
            CTokenKind::Decimal128Keyword => write!(f, "'_Decimal128' keyword"),
            CTokenKind::Decimal32Keyword => write!(f, "'_Decimal32' keyword"),
            CTokenKind::Decimal64Keyword => write!(f, "'_Decimal64' keyword"),
            CTokenKind::GenericKeyword => write!(f, "'_Generic' keyword"),
            CTokenKind::ImaginaryKeyword => write!(f, "'_Imaginary' keyword"),
            CTokenKind::NoreturnKeyword => write!(f, "'_Noreturn' keyword"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Invalid {
    UniversalCharacterNameNotSupported,
    UnrecognizedSymbol,
    UnrepresentableInteger,
}

impl Display for Invalid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Invalid::UniversalCharacterNameNotSupported => {
                write!(f, "unsupported universal character name")
            }
            Invalid::UnrecognizedSymbol => write!(f, "unrecognized symbol"),
            Invalid::UnrepresentableInteger => write!(f, "unrepresentable integer literal"),
        }
    }
}
