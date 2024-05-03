use crate::line_column::Location;
use derive_more::{Deref, IsVariant, Unwrap};

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
