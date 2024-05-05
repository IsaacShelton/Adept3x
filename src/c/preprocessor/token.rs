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
pub enum Encoding {
    Default,
    Utf8,  // 'u8'
    Utf16, // 'u'
    Utf32, // 'U'
    Wide,  // 'L'
}

#[derive(Clone, Debug)]
pub enum PreTokenKind {
    HeaderName(String),
    Identifier(String),
    Number(String),
    CharacterConstant(Encoding, String),
    StringLiteral(Encoding, String),
    Punctuator(Punctuator),
    UniversalCharacterName(char), // e.g. '\u1F3E'
    Other(char),
}

impl PreToken {
    pub fn is_hash(&self) -> bool {
        match self.kind {
            PreTokenKind::Punctuator(Punctuator::Hash) => true,
            _ => false,
        }
    }

    pub fn is_identifier(&self, content: &str) -> bool {
        match &self.kind {
            PreTokenKind::Identifier(identifier) if content == identifier => true,
            _ => false,
        }
    }

    pub fn is_open_paren_disregard_whitespace(&self) -> bool {
        match self.kind {
            PreTokenKind::Punctuator(Punctuator::OpenParen { .. }) => true,
            _ => false,
        }
    }
}

// We don't support the use of digraphs. e.g. '<:', ':>', '<%', '%>', '%:', '%:%:'
// (nor trigraphs, as they were removed in C23)
#[derive(Clone, Debug)]
pub enum Punctuator {
    OpenBracket,
    CloseBracket,
    OpenParen { preceeded_by_whitespace: bool },
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
    Add,
    Subtract,
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
