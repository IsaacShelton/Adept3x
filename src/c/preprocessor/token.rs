#[derive(Clone, Debug)]
pub struct PreToken {
    pub kind: PreTokenKind,
}

impl PreToken {
    pub fn new(kind: PreTokenKind) -> Self {
        Self { kind }
    }
}

#[derive(Clone, Debug)]
pub enum PreTokenKind {
    HeaderName(String),
    Identifier(String),
    Number(String),
    CharacterConstant(String),
    StringLiteral(String),
    Punctuator(Punctuator),
    UniversalCharacterName(char), // e.g. '\u1F3E'
    Other(char),
}

// We don't support the use of digraphs. e.g. '<:', ':>', '<%', '%>', '%:', '%:%:'
// (nor trigraphs, as they were removed in C23)
#[derive(Clone, Debug)]
pub enum Punctuator {
    OpenBracket,
    CloseBracket,
    OpenParen,
    CloseParen,
    OpenCurly,
    CloseCurly,
    Comma,
    Colon,
    Semicolon,
    Multiply,
    Assign,
    Ellipses,
    Hash,
    Dot,
    Arrow,
    Increment,
    Decrement,
    HashConcat,
    Ampersand,
    Plus,
    Minus,
    BitComplement,
    Not,
    Divide,
    Modulus,
    LeftShift,
    RightShift,
    NotEquals,
    LessThan,
    GreaterThan,
    LessThanEq,
    GreaterThanEq,
    DoubleEquals,
    BitXor,
    BitOr,
    LogicalAnd,
    LogicalOr,
    Ternary,
    MultiplyAssign,
    DivideAssign,
    ModulusAssign,
    AddAssign,
    SubtractAssign,
    LeftShiftAssign,
    RightShiftAssign,
    BitAndAssign,
    BitXorAssign,
    BitOrAssign,
}
