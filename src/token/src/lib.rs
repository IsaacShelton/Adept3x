mod directive;
mod punct;
mod string;

use derivative::Derivative;
use derive_more::{Deref, IsVariant, PartialEq, Unwrap};
pub use directive::Directive;
use lazy_static::lazy_static;
use num_bigint::BigInt;
pub use punct::Punct;
use std::fmt::{Debug, Display};
pub use string::{StringLiteral, StringModifier};
use util_infinite_iterator::IsEnd;
use util_text::{ColumnSpacingAtom, LineSpacingAtom};

#[derive(Clone, Debug, Deref, Derivative)]
#[derivative(PartialEq)]
pub struct Token<S> {
    #[deref]
    pub kind: TokenKind,

    #[derivative(PartialEq = "ignore")]
    pub source: S,
}

impl<S> Token<S> {
    pub fn new(kind: TokenKind, source: S) -> Self {
        Token { kind, source }
    }

    pub fn is_end_of_file(&self) -> bool {
        self.kind.is_end_of_file()
    }

    pub fn is_assignment_like(&self) -> bool {
        self.kind.is_assignment_like()
    }
}

impl<S> IsEnd for Token<S> {
    fn is_end(&self) -> bool {
        self.kind.is_end_of_file()
    }
}

#[derive(Clone, Debug, PartialEq, IsVariant, Unwrap)]
pub enum TokenKind {
    EndOfFile,
    Error(char),
    ColumnSpacing(ColumnSpacingAtom),
    LineSpacing(LineSpacingAtom),
    Identifier(String),
    Polymorph(String),
    Grouping(char),
    String(StringLiteral),
    MissingStringTermination,
    Integer(BigInt, usize),
    Float(f64, usize),
    Directive(Directive),
    Punct(Punct),
    Label(String),
}

impl Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EndOfFile => f.write_str("end-of-file"),
            Self::Error(error_c) => write!(f, "{}", *error_c),
            Self::ColumnSpacing(atom) => write!(f, "{}", atom),
            Self::LineSpacing(atom) => write!(f, "{}", atom),
            Self::Identifier(name) => write!(f, "{}", name),
            Self::Polymorph(name) => write!(f, "${}", name),
            Self::Grouping(c) => write!(f, "{}", c),
            Self::String(string_literal) => write!(f, "{}", string_literal),
            Self::MissingStringTermination => Ok(()),
            Self::Integer(value, _) => write!(f, "{}", value),
            Self::Float(value, _) => write!(f, "{}", value),
            Self::Directive(directive) => write!(f, "{}", directive),
            Self::Punct(punct) => write!(f, "{}", punct),
            Self::Label(name) => write!(f, "@{}@", name),
        }
    }
}

const ASSIGNMENT_OPERATORS: &[&'static str] = &[
    "=", ":=", "+=", "-=", "*=", "/=", "%=", "&=", "^=", "|=", "<<=", "<<<=", ">>=", ">>>=",
];

#[allow(unused)]
const NON_ASSIGNMENT_OPERATORS: &[&'static str] = &[
    ",", ".", ":", "::", "++", "--", "!", "~", "*", "/", "%", "+", "-", "<<", "<<<", ">>", ">>>",
    "<", "<=", ">", ">=", "==", "!=", "&", "^", "|", "&&", "||",
];

pub const ALL_DIRECTIVES: &[&'static str] = &["fn", "type", "struct", "enum", "record"];

// Since Rust's const evaluation sucks
lazy_static! {
    pub static ref ALL_PUNCT_SORTED: &'static [&'static str] = {
        let mut puncts = Vec::<&'static str>::new();
        puncts.extend(ASSIGNMENT_OPERATORS);
        puncts.extend(NON_ASSIGNMENT_OPERATORS);
        puncts.sort_by_key(|s| s.len());
        puncts.reverse();
        Vec::leak(puncts)
    };
}

impl TokenKind {
    pub fn precedence(&self) -> usize {
        match self {
            Self::Grouping('{' | '[') => 16,
            Self::Punct(c) if c.is(".") => 16,
            Self::Punct(c) if c.is_any(&["++", "--"]) => 15,
            Self::Punct(c) if c.is_any(&["!", "~"]) => 14,
            Self::Punct(c) if c.is_any(&["*", "/", "%"]) => 12,
            Self::Punct(c) if c.is_any(&["+", "-"]) => 11,
            Self::Punct(c) if c.is_any(&["<<", "<<<", ">>", ">>>"]) => 10,
            Self::Punct(c) if c.is_any(&["<", "<=", ">", ">="]) => 9,
            Self::Punct(c) if c.is_any(&["==", "!="]) => 8,
            Self::Punct(c) if c.is_any(&["&"]) => 7,
            Self::Punct(c) if c.is_any(&["^"]) => 6,
            Self::Punct(c) if c.is_any(&["|"]) => 5,
            Self::Punct(c) if c.is_any(&["&&"]) => 4,
            Self::Punct(c) if c.is_any(&["||"]) => 3,
            Self::Punct(c) if c.is_any(ASSIGNMENT_OPERATORS) => 1,
            Self::Punct(c) if c.is("::") => 0,
            Self::EndOfFile
            | Self::Error(_)
            | Self::LineSpacing(_)
            | Self::ColumnSpacing(_)
            | Self::Identifier(_)
            | Self::Polymorph(_)
            | Self::Grouping(_)
            | Self::String { .. }
            | Self::MissingStringTermination
            | Self::Integer { .. }
            | Self::Float { .. }
            | Self::Directive(_)
            | Self::Punct(_)
            | Self::Label(_) => 0,
        }
    }

    pub fn len(&self) -> usize {
        match self {
            TokenKind::EndOfFile => 0,
            TokenKind::Error(_c) => 1,
            TokenKind::ColumnSpacing(atom) => atom.len() as usize,
            TokenKind::LineSpacing(atom) => atom.count,
            TokenKind::Identifier(name) => name.len(),
            TokenKind::Polymorph(name) => 1 + name.len(),
            TokenKind::Grouping(_) => 1,
            TokenKind::String(string) => string.literal.len(),
            TokenKind::MissingStringTermination => 0,
            TokenKind::Integer(_, len) => *len,
            TokenKind::Float(_, len) => *len,
            TokenKind::Directive(directive) => directive.len_with_prefix(),
            TokenKind::Punct(punct) => punct.len(),
            TokenKind::Label(_) => todo!(),
        }
    }

    pub fn is_assignment_like(&self) -> bool {
        match self {
            Self::Punct(c) if c.is_any(ASSIGNMENT_OPERATORS) => true,
            _ => false,
        }
    }

    pub fn at<S>(self, source: S) -> Token<S> {
        Token { kind: self, source }
    }
}
