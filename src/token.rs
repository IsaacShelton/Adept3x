use crate::{inflow::InflowEnd, source_files::Source};
use derivative::Derivative;
use derive_more::{Deref, IsVariant, Unwrap};
use num_bigint::BigInt;
use std::fmt::Display;

#[derive(Clone, Debug, Deref, Derivative)]
#[derivative(PartialEq)]
pub struct Token {
    #[deref]
    pub kind: TokenKind,

    #[derivative(PartialEq = "ignore")]
    pub source: Source,
}

impl Token {
    pub fn new(kind: TokenKind, source: Source) -> Token {
        Token { kind, source }
    }

    pub fn is_end_of_file(&self) -> bool {
        self.kind.is_end_of_file()
    }

    pub fn is_assignment_like(&self) -> bool {
        self.kind.is_assignment_like()
    }
}

impl InflowEnd for Token {
    fn is_inflow_end(&self) -> bool {
        self.kind.is_end_of_file()
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

#[derive(Clone, Debug, PartialEq)]
pub struct NamespacedIndentifier {
    pub namespace: String,
    pub basename: String,
}

#[derive(Clone, Debug, PartialEq, IsVariant, Unwrap)]
pub enum TokenKind {
    EndOfFile,
    Error(String),
    Newline,
    Identifier(String),
    NamespacedIdentifier(NamespacedIndentifier),
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
    TypeAliasKeyword,
    IfKeyword,
    ElseKeyword,
    ElifKeyword,
    WhileKeyword,
    TrueKeyword,
    FalseKeyword,
    DefineKeyword,
    ZeroedKeyword,
    PragmaKeyword,
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
    AddressOf,
    Dereference,
    DeclareAssign,
    Assign,
    AddAssign,
    SubtractAssign,
    MultiplyAssign,
    DivideAssign,
    ModulusAssign,
    BitAndAssign,
    BitOrAssign,
    BitXorAssign,
    LeftShiftAssign,
    RightShiftAssign,
    LogicalLeftShiftAssign,
    LogicalRightShiftAssign,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    LeftShift,
    RightShift,
    LogicalLeftShift,
    LogicalRightShift,
    Increment,
    Decrement,
    Namespace,
    Extend,
    FatArrow,
}

impl Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenKind::EndOfFile => f.write_str("end-of-file"),
            TokenKind::Error(message) => write!(f, "'lex error - {}'", message),
            TokenKind::Newline => f.write_str("'newline'"),
            TokenKind::Identifier(_) => f.write_str("'identifier'"),
            TokenKind::NamespacedIdentifier(_) => f.write_str("'namespaced identifier'"),
            TokenKind::OpenCurly => f.write_str("'{'"),
            TokenKind::CloseCurly => f.write_str("'}'"),
            TokenKind::OpenParen => f.write_str("'('"),
            TokenKind::CloseParen => f.write_str("')'"),
            TokenKind::OpenBracket => f.write_str("'['"),
            TokenKind::CloseBracket => f.write_str("']'"),
            TokenKind::String { .. } => f.write_str("'string'"),
            TokenKind::Integer { .. } => f.write_str("'integer'"),
            TokenKind::Float { .. } => f.write_str("'float'"),
            TokenKind::DocComment(_) => f.write_str("'documentation comment'"),
            TokenKind::FuncKeyword => f.write_str("'func' keyword"),
            TokenKind::ReturnKeyword => f.write_str("'return' keyword"),
            TokenKind::StructKeyword => f.write_str("'struct' keyword"),
            TokenKind::UnionKeyword => f.write_str("'union' keyword"),
            TokenKind::EnumKeyword => f.write_str("'enum' keyword"),
            TokenKind::TypeAliasKeyword => f.write_str("'typealias' keyword"),
            TokenKind::IfKeyword => f.write_str("'if' keyword"),
            TokenKind::ElseKeyword => f.write_str("'else' keyword"),
            TokenKind::ElifKeyword => f.write_str("'elif' keyword"),
            TokenKind::WhileKeyword => f.write_str("'while' keyword"),
            TokenKind::TrueKeyword => f.write_str("'true'"),
            TokenKind::FalseKeyword => f.write_str("'false'"),
            TokenKind::DefineKeyword => f.write_str("'define' keyword"),
            TokenKind::ZeroedKeyword => f.write_str("'zeroed' keyword"),
            TokenKind::PragmaKeyword => f.write_str("'pragma' keyword"),
            TokenKind::Member => f.write_str("'.'"),
            TokenKind::Add => f.write_str("'+'"),
            TokenKind::Subtract => f.write_str("'-'"),
            TokenKind::Multiply => f.write_str("'*'"),
            TokenKind::Divide => f.write_str("'/'"),
            TokenKind::Modulus => f.write_str("'%'"),
            TokenKind::Equals => f.write_str("'=='"),
            TokenKind::NotEquals => f.write_str("'!='"),
            TokenKind::LessThan => f.write_str("'<'"),
            TokenKind::GreaterThan => f.write_str("'>'"),
            TokenKind::LessThanEq => f.write_str("'<='"),
            TokenKind::GreaterThanEq => f.write_str("'>='"),
            TokenKind::OpenAngle => f.write_str("open angle '<'"),
            TokenKind::Not => f.write_str("'!'"),
            TokenKind::BitComplement => f.write_str("'~'"),
            TokenKind::Comma => f.write_str("','"),
            TokenKind::Colon => f.write_str("':'"),
            TokenKind::Hash => f.write_str("'#'"),
            TokenKind::Ellipsis => f.write_str("'...'"),
            TokenKind::AddressOf => f.write_str("(address of) '&'"),
            TokenKind::Dereference => f.write_str("(dereference) '*'"),
            TokenKind::DeclareAssign => f.write_str("':='"),
            TokenKind::Assign => f.write_str("'='"),
            TokenKind::AddAssign => f.write_str("'+='"),
            TokenKind::SubtractAssign => f.write_str("'-='"),
            TokenKind::MultiplyAssign => f.write_str("'*='"),
            TokenKind::DivideAssign => f.write_str("'/='"),
            TokenKind::ModulusAssign => f.write_str("'%='"),
            TokenKind::BitAndAssign => f.write_str("'&='"),
            TokenKind::BitOrAssign => f.write_str("'|='"),
            TokenKind::BitXorAssign => f.write_str("'^='"),
            TokenKind::LeftShiftAssign => f.write_str("'<<='"),
            TokenKind::RightShiftAssign => f.write_str("'>>='"),
            TokenKind::LogicalLeftShiftAssign => f.write_str("'<<<='"),
            TokenKind::LogicalRightShiftAssign => f.write_str("'>>>='"),
            TokenKind::And => f.write_str("'&&'"),
            TokenKind::Or => f.write_str("'||'"),
            TokenKind::BitAnd => f.write_str("'&'"),
            TokenKind::BitOr => f.write_str("'|'"),
            TokenKind::BitXor => f.write_str("'^'"),
            TokenKind::LeftShift => f.write_str("'<<'"),
            TokenKind::RightShift => f.write_str("'>>'"),
            TokenKind::LogicalLeftShift => f.write_str("'<<<'"),
            TokenKind::LogicalRightShift => f.write_str("'>>>'"),
            TokenKind::Increment => f.write_str("'++'"),
            TokenKind::Decrement => f.write_str("'--'"),
            TokenKind::Namespace => f.write_str("'::'"),
            TokenKind::Extend => f.write_str("'..'"),
            TokenKind::FatArrow => f.write_str("'=>'"),
        }
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
            TokenKind::BitAnd => 7,
            TokenKind::BitXor => 6,
            TokenKind::BitOr => 5,
            TokenKind::And => 4,
            TokenKind::Or => 3,
            TokenKind::DeclareAssign
            | TokenKind::AddAssign
            | TokenKind::SubtractAssign
            | TokenKind::MultiplyAssign
            | TokenKind::DivideAssign
            | TokenKind::ModulusAssign
            | TokenKind::BitAndAssign
            | TokenKind::BitOrAssign
            | TokenKind::BitXorAssign
            | TokenKind::LeftShiftAssign
            | TokenKind::RightShiftAssign
            | TokenKind::LogicalLeftShiftAssign
            | TokenKind::LogicalRightShiftAssign
            | TokenKind::Assign => 1,

            TokenKind::EndOfFile
            | TokenKind::Error(_)
            | TokenKind::Newline
            | TokenKind::Identifier(_)
            | TokenKind::NamespacedIdentifier(_)
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
            | TokenKind::TypeAliasKeyword
            | TokenKind::IfKeyword
            | TokenKind::ElseKeyword
            | TokenKind::ElifKeyword
            | TokenKind::WhileKeyword
            | TokenKind::TrueKeyword
            | TokenKind::FalseKeyword
            | TokenKind::DefineKeyword
            | TokenKind::ZeroedKeyword
            | TokenKind::PragmaKeyword
            | TokenKind::OpenAngle
            | TokenKind::Comma
            | TokenKind::Colon
            | TokenKind::Hash
            | TokenKind::Ellipsis
            | TokenKind::AddressOf
            | TokenKind::Dereference
            | TokenKind::Namespace
            | TokenKind::Extend
            | TokenKind::FatArrow => 0,
        }
    }

    pub fn is_assignment_like(&self) -> bool {
        matches!(
            self,
            Self::AddAssign
                | Self::SubtractAssign
                | Self::MultiplyAssign
                | Self::DivideAssign
                | Self::ModulusAssign
                | Self::BitAndAssign
                | Self::BitOrAssign
                | Self::BitXorAssign
                | Self::LeftShiftAssign
                | Self::RightShiftAssign
                | Self::LogicalLeftShiftAssign
                | Self::LogicalRightShiftAssign
                | Self::Assign
        )
    }

    pub fn could_start_type(&self) -> bool {
        matches!(self, TokenKind::Identifier(_))
    }

    pub fn at(self, source: Source) -> Token {
        Token { kind: self, source }
    }
}
