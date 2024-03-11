use crate::line_column::Location;
use derive_more::{IsVariant, Deref};
use num_bigint::BigInt;
use std::fmt::Display;

#[derive(Clone, Debug, PartialEq, Deref)]
pub struct TokenInfo {
    #[deref]
    pub token: Token,

    pub location: Location,
}

impl TokenInfo {
    pub fn new(token: Token, location: Location) -> TokenInfo {
        TokenInfo { token, location }
    }

    pub fn is_end_of_file(&self) -> bool {
        match self.token {
            Token::EndOfFile => true,
            _ => false,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum StringModifier {
    Normal,
    NullTerminated,
}

#[derive(Clone, Debug, PartialEq, IsVariant)]
pub enum Token {
    EndOfFile,
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
    Comma,
    Colon,
    Hash,
    Ellipsis,
    DeclareAssign,
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Token::EndOfFile => "end-of-file",
            Token::Error(_) => "'error'",
            Token::Newline => "'newline'",
            Token::Identifier(_) => "'identifier'",
            Token::OpenCurly => "'{'",
            Token::CloseCurly => "'}'",
            Token::OpenParen => "'('",
            Token::CloseParen => "')'",
            Token::OpenBracket => "']'",
            Token::CloseBracket => "']'",
            Token::String { .. } => "'string'",
            Token::Integer { .. } => "'integer'",
            Token::Float { .. } => "'float'",
            Token::DocComment(_) => "'documentation comment'",
            Token::FuncKeyword => "'func' keyword",
            Token::ReturnKeyword => "'return' keyword",
            Token::Member => "'.'",
            Token::Add => "'+'",
            Token::Subtract => "'-'",
            Token::Multiply => "'*'",
            Token::Divide => "'/'",
            Token::Modulus => "'%'",
            Token::Equals => "'='",
            Token::NotEquals => "'!='",
            Token::LessThan => "'<'",
            Token::GreaterThan => "'>'",
            Token::LessThanEq => "'<='",
            Token::GreaterThanEq => "'>='",
            Token::Not => "'!'",
            Token::Comma => "','",
            Token::Colon => "':'",
            Token::Hash => "'#'",
            Token::Ellipsis => "'...'",
            Token::DeclareAssign => "':='",
        })
    }
}
