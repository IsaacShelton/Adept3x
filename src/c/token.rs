use super::{encoding::Encoding, punctuator::Punctuator};
use crate::line_column::Location;
use derive_more::{Deref, IsVariant, Unwrap};
use num_bigint::BigInt;

#[derive(Clone, Debug, PartialEq, IsVariant, Unwrap)]
pub enum CTokenKind {
    EndOfFile,
    Identifier(String),
    AutoKeyword,
    BreakKeyword,
    CaseKeyword,
    CharKeyword,
    ConstKeyword,
    ContinueKeyword,
    DefaultKeyword,
    DoKeyword,
    DoubleKeyword,
    ElseKeyword,
    EnumKeyword,
    ExternKeyword,
    FloatKeyword,
    ForKeyword,
    GotoKeyword,
    IfKeyword,
    IntKeyword,
    LongKeyword,
    RegisterKeyword,
    ReturnKeyword,
    ShortKeyword,
    SignedKeyword,
    SizeofKeyword,
    StaticKeyword,
    StructKeyword,
    SwitchKeyword,
    TypedefKeyword,
    UnionKeyword,
    UnsignedKeyword,
    VoidKeyword,
    VolatileKeyword,
    WhileKeyword,
    Punctuator(Punctuator),
    Integer(BigInt, IntegerSuffix),
    Float(f64, FloatSuffix),
    CharacterConstant(Encoding, String),
    StringLiteral(Encoding, String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum IntegerSuffix {
    Regular,
    Unsigned,
    Long,
    UnsignedLong,
    LongLong,
    UnsignedLongLong,
    BigInteger,
    UnsignedBigInteger,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FloatSuffix {
    Float,
    LongDouble,
    Decimal32,
    Decimal64,
    Decimal128,
}

#[derive(Clone, Debug, PartialEq, Deref)]
pub struct CToken {
    #[deref]
    pub kind: CTokenKind,

    pub location: Location,
}

impl CToken {
    pub fn new(kind: CTokenKind, location: Location) -> CToken {
        CToken { kind, location }
    }
}
