use crate::line_column::Location;
use derive_more::{Deref, IsVariant, Unwrap};
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
        self.kind.is_end_of_file()
    }

    pub fn is_assignment_like(&self) -> bool {
        self.kind.is_assignment_like()
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum StringModifier {
    Normal,
    NullTerminated,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StringLiteral {
    pub value: String,
    pub modifier: StringModifier,
}

#[derive(Clone, Debug, PartialEq, IsVariant, Unwrap)]
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
    String(StringLiteral),
    Integer(BigInt),
    Float(f64),
    DocComment(String),
    FuncKeyword,
    ReturnKeyword,
    StructKeyword,
    UnionKeyword,
    EnumKeyword,
    AliasKeyword,
    IfKeyword,
    ElseKeyword,
    ElifKeyword,
    WhileKeyword,
    TrueKeyword,
    FalseKeyword,
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
    BitComplement,
    Comma,
    Colon,
    Hash,
    Ellipsis,
    DeclareAssign,
    Assign,
    AddAssign,
    SubtractAssign,
    MultiplyAssign,
    DivideAssign,
    ModulusAssign,
    AmpersandAssign,
    PipeAssign,
    CaretAssign,
    LeftShiftAssign,
    RightShiftAssign,
    LogicalLeftShiftAssign,
    LogicalRightShiftAssign,
    And,
    Or,
    Ampersand,
    Pipe,
    Caret,
    LeftShift,
    RightShift,
    LogicalLeftShift,
    LogicalRightShift,
    Increment,
    Decrement,
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
            TokenKind::OpenBracket => "'['",
            TokenKind::CloseBracket => "']'",
            TokenKind::String { .. } => "'string'",
            TokenKind::Integer { .. } => "'integer'",
            TokenKind::Float { .. } => "'float'",
            TokenKind::DocComment(_) => "'documentation comment'",
            TokenKind::FuncKeyword => "'func' keyword",
            TokenKind::ReturnKeyword => "'return' keyword",
            TokenKind::StructKeyword => "'struct' keyword",
            TokenKind::UnionKeyword => "'union' keyword",
            TokenKind::EnumKeyword => "'enum' keyword",
            TokenKind::AliasKeyword => "'alias' keyword",
            TokenKind::IfKeyword => "'if' keyword",
            TokenKind::ElseKeyword => "'else' keyword",
            TokenKind::ElifKeyword => "'elif' keyword",
            TokenKind::WhileKeyword => "'while' keyword",
            TokenKind::TrueKeyword => "'true'",
            TokenKind::FalseKeyword => "'false'",
            TokenKind::Member => "'.'",
            TokenKind::Add => "'+'",
            TokenKind::Subtract => "'-'",
            TokenKind::Multiply => "'*'",
            TokenKind::Divide => "'/'",
            TokenKind::Modulus => "'%'",
            TokenKind::Equals => "'=='",
            TokenKind::NotEquals => "'!='",
            TokenKind::LessThan => "'<'",
            TokenKind::GreaterThan => "'>'",
            TokenKind::LessThanEq => "'<='",
            TokenKind::GreaterThanEq => "'>='",
            TokenKind::OpenAngle => "open angle '<'",
            TokenKind::Not => "'!'",
            TokenKind::BitComplement => "'~'",
            TokenKind::Comma => "','",
            TokenKind::Colon => "':'",
            TokenKind::Hash => "'#'",
            TokenKind::Ellipsis => "'...'",
            TokenKind::DeclareAssign => "':='",
            TokenKind::Assign => "'='",
            TokenKind::AddAssign => "'+='",
            TokenKind::SubtractAssign => "'-='",
            TokenKind::MultiplyAssign => "'*='",
            TokenKind::DivideAssign => "'/='",
            TokenKind::ModulusAssign => "'%='",
            TokenKind::AmpersandAssign => "'&='",
            TokenKind::PipeAssign => "'|='",
            TokenKind::CaretAssign => "'^='",
            TokenKind::LeftShiftAssign => "'<<='",
            TokenKind::RightShiftAssign => "'>>='",
            TokenKind::LogicalLeftShiftAssign => "'<<<='",
            TokenKind::LogicalRightShiftAssign => "'>>>='",
            TokenKind::And => "'&&'",
            TokenKind::Or => "'||'",
            TokenKind::Ampersand => "'&'",
            TokenKind::Pipe => "'|'",
            TokenKind::Caret => "'^'",
            TokenKind::LeftShift => "'<<'",
            TokenKind::RightShift => "'>>'",
            TokenKind::LogicalLeftShift => "'<<<'",
            TokenKind::LogicalRightShift => "'>>>'",
            TokenKind::Increment => "'++'",
            TokenKind::Decrement => "'--'",
        })
    }
}

impl TokenKind {
    pub fn precedence(&self) -> usize {
        match self {
            TokenKind::OpenCurly => 16,
            TokenKind::OpenBracket => 16,
            TokenKind::Member => 16,
            TokenKind::Increment => 15,
            TokenKind::Decrement => 15,
            TokenKind::Not => 14,
            TokenKind::BitComplement => 14,
            TokenKind::Multiply => 12,
            TokenKind::Divide => 12,
            TokenKind::Modulus => 12,
            TokenKind::Add => 11,
            TokenKind::Subtract => 11,
            TokenKind::LeftShift => 10,
            TokenKind::RightShift => 10,
            TokenKind::LogicalLeftShift => 10,
            TokenKind::LogicalRightShift => 10,
            TokenKind::LessThan => 9,
            TokenKind::GreaterThan => 9,
            TokenKind::LessThanEq => 9,
            TokenKind::GreaterThanEq => 9,
            TokenKind::Equals => 8,
            TokenKind::NotEquals => 8,
            TokenKind::Ampersand => 7,
            TokenKind::Caret => 6,
            TokenKind::Pipe => 5,
            TokenKind::And => 4,
            TokenKind::Or => 3,
            TokenKind::DeclareAssign
            | TokenKind::AddAssign
            | TokenKind::SubtractAssign
            | TokenKind::MultiplyAssign
            | TokenKind::DivideAssign
            | TokenKind::ModulusAssign
            | TokenKind::AmpersandAssign
            | TokenKind::PipeAssign
            | TokenKind::CaretAssign
            | TokenKind::LeftShiftAssign
            | TokenKind::RightShiftAssign
            | TokenKind::LogicalLeftShiftAssign
            | TokenKind::LogicalRightShiftAssign
            | TokenKind::Assign => 1,

            TokenKind::EndOfFile
            | TokenKind::Error(_)
            | TokenKind::Newline
            | TokenKind::Identifier(_)
            | TokenKind::CloseCurly
            | TokenKind::OpenParen
            | TokenKind::CloseParen
            | TokenKind::CloseBracket
            | TokenKind::String { .. }
            | TokenKind::Integer { .. }
            | TokenKind::Float { .. }
            | TokenKind::DocComment(_)
            | TokenKind::FuncKeyword
            | TokenKind::ReturnKeyword
            | TokenKind::StructKeyword
            | TokenKind::UnionKeyword
            | TokenKind::EnumKeyword
            | TokenKind::AliasKeyword
            | TokenKind::IfKeyword
            | TokenKind::ElseKeyword
            | TokenKind::ElifKeyword
            | TokenKind::WhileKeyword
            | TokenKind::TrueKeyword
            | TokenKind::FalseKeyword
            | TokenKind::OpenAngle
            | TokenKind::Comma
            | TokenKind::Colon
            | TokenKind::Hash
            | TokenKind::Ellipsis => 0,
        }
    }

    pub fn is_assignment_like(&self) -> bool {
        match self {
            Self::AddAssign
            | Self::SubtractAssign
            | Self::MultiplyAssign
            | Self::DivideAssign
            | Self::ModulusAssign
            | Self::AmpersandAssign
            | Self::PipeAssign
            | Self::CaretAssign
            | Self::LeftShiftAssign
            | Self::RightShiftAssign
            | Self::LogicalLeftShiftAssign
            | Self::LogicalRightShiftAssign
            | Self::Assign => true,
            _ => false,
        }
    }

    pub fn could_start_type(&self) -> bool {
        match self {
            TokenKind::Identifier(_)
            | TokenKind::StructKeyword
            | TokenKind::UnionKeyword
            | TokenKind::EnumKeyword => true,
            _ => false,
        }
    }

    pub fn at(self, location: Location) -> Token {
        Token {
            kind: self,
            location,
        }
    }
}
