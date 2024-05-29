use super::{encoding::Encoding, lexer::LexError, punctuator::Punctuator};
use crate::ast::Source;
use derive_more::{Deref, IsVariant, Unwrap};

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
    pub fn is_open_paren(&self) -> bool {
        match self {
            CTokenKind::Punctuator(Punctuator::OpenParen { .. }) => true,
            _ => false,
        }
    }

    pub fn precedence(&self) -> usize {
        todo!("c token precedence")
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

#[derive(Clone, Debug, PartialEq)]
pub enum IntegerSuffix {
    Int,
    UnsignedInt,
    Long,
    UnsignedLong,
    LongLong,
    UnsignedLongLong,
}

#[derive(Clone, Debug, PartialEq)]
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
