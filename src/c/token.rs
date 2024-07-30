use super::{encoding::Encoding, lexer::LexError, punctuator::Punctuator};
use crate::{ast::Source, inflow::InflowEnd};
use derive_more::{Deref, IsVariant, Unwrap};
use num_bigint::BigInt;

#[derive(Clone, Debug, PartialEq, IsVariant, Unwrap)]
pub enum CTokenKind {
    EndOfFile,
    LexError(LexError),
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
