use crate::line_column::Location;
use derive_more::{Deref, IsVariant};
use num_bigint::BigInt;
use std::fmt::Display;

#[derive(Clone, Debug, PartialEq, Deref)]
pub struct Token {
    #[deref]
    pub kind: TokenKind,

    pub location: Location,
}

impl Token {
    pub fn new(kind: TokenKind, location: Location) -> Token {
        Token { kind, location }
    }

    pub fn is_end_of_file(&self) -> bool {
        match self.kind {
            TokenKind::EndOfFile => true,
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
pub enum TokenKind {
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
    OpenAngle,
    Not,
    Comma,
    Colon,
    Hash,
    Ellipsis,
    DeclareAssign,
}

impl Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            TokenKind::EndOfFile => "end-of-file",
            TokenKind::Error(_) => "'error'",
            TokenKind::Newline => "'newline'",
            TokenKind::Identifier(_) => "'identifier'",
            TokenKind::OpenCurly => "'{'",
            TokenKind::CloseCurly => "'}'",
            TokenKind::OpenParen => "'('",
            TokenKind::CloseParen => "')'",
            TokenKind::OpenBracket => "']'",
            TokenKind::CloseBracket => "']'",
            TokenKind::String { .. } => "'string'",
            TokenKind::Integer { .. } => "'integer'",
            TokenKind::Float { .. } => "'float'",
            TokenKind::DocComment(_) => "'documentation comment'",
            TokenKind::FuncKeyword => "'func' keyword",
            TokenKind::ReturnKeyword => "'return' keyword",
            TokenKind::Member => "'.'",
            TokenKind::Add => "'+'",
            TokenKind::Subtract => "'-'",
            TokenKind::Multiply => "'*'",
            TokenKind::Divide => "'/'",
            TokenKind::Modulus => "'%'",
            TokenKind::Equals => "'='",
            TokenKind::NotEquals => "'!='",
            TokenKind::LessThan => "'<'",
            TokenKind::GreaterThan => "'>'",
            TokenKind::LessThanEq => "'<='",
            TokenKind::GreaterThanEq => "'>='",
            TokenKind::OpenAngle => "open angle '<'",
            TokenKind::Not => "'!'",
            TokenKind::Comma => "','",
            TokenKind::Colon => "':'",
            TokenKind::Hash => "'#'",
            TokenKind::Ellipsis => "'...'",
            TokenKind::DeclareAssign => "':='",
        })
    }
}

impl TokenKind {
    pub fn precedence(&self) -> usize {
        match self {
            TokenKind::OpenCurly => 16,
            TokenKind::OpenBracket => 16,
            TokenKind::Member => 16,
            TokenKind::Multiply => 12,
            TokenKind::Divide => 12,
            TokenKind::Modulus => 12,
            TokenKind::Not => 14,
            TokenKind::Add => 11,
            TokenKind::Subtract => 11,
            TokenKind::LessThan => 9,
            TokenKind::GreaterThan => 9,
            TokenKind::LessThanEq => 9,
            TokenKind::GreaterThanEq => 9,
            TokenKind::Equals => 8,
            TokenKind::NotEquals => 8,
            TokenKind::DeclareAssign => 1,
            _ => 0,
        }
    }
}
