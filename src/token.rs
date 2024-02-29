use std::fmt::Display;

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

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Token::*;
        f.write_str(match self {
            Error(_) => "'error'",
            Newline => "'newline'",
            Identifier(_) => "'identifier'",
            OpenCurly => "'{'",
            CloseCurly => "'}'",
            OpenParen => "'('",
            CloseParen => "')'",
            OpenBracket => "']'",
            CloseBracket => "']'",
            String { .. } => "'string'",
            Integer { .. } => "'integer'",
            Float { .. } => "'float'",
            DocComment(_) => "'documentation comment'",
            FuncKeyword => "'func' keyword",
            ReturnKeyword => "'return' keyword",
            Member => "'.'",
            Add => "'+'",
            Subtract => "'-'",
            Multiply => "'*'",
            Divide => "'/'",
            Modulus => "'%'",
            Equals => "'='",
            NotEquals => "'!='",
            LessThan => "'<'",
            GreaterThan => "'>'",
            LessThanEq => "'<='",
            GreaterThanEq => "'>='",
            Not => "'!'",
        })
    }
}

