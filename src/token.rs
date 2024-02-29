use num_bigint::BigInt;

use crate::line_column::Location;

#[derive(Clone, Debug, PartialEq)]
pub struct TokenInfo {
    pub token: Token,
    pub location: Location,
}

impl TokenInfo {
    pub fn new(token: Token, location: Location) -> TokenInfo {
        TokenInfo { token, location }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum StringModifier {
    Normal,
    NullTerminated,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    Error(String),
    Newline,
    Identifier(String),
    OpenCurly,
    CloseCurly,
    OpenParen,
    CloseParen,
    OpenBracket,
    CloseBracket,
    String {
        value: String,
        modifier: StringModifier,
    },
    Integer {
        value: BigInt,
    },
    Float {
        value: f64,
    },
    DocComment(String),
    FuncKeyword,
    ReturnKeyword,
    Member,
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulus,
    Equals,
    NotEquals,
    LessThan,
    GreaterThan,
    LessThanEq,
    GreaterThanEq,
    Not,
}
