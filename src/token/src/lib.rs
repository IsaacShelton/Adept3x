use derivative::Derivative;
use derive_more::{Deref, IsVariant, PartialEq, Unwrap};
use num_bigint::BigInt;
use std::fmt::{Debug, Display};
use util_infinite_iterator::IsEnd;
use util_text::ColumnSpacingAtom;

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

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum StringModifier {
    Normal,
    Character,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StringLiteral {
    pub literal: String,
}

impl StringLiteral {
    pub fn modifier(&self) -> StringModifier {
        if self.literal.starts_with('"') {
            return StringModifier::Normal;
        }

        if self.literal.starts_with('\'') {
            return StringModifier::Character;
        }

        panic!("Invalid string literal")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Directive {
    Standard(&'static str),
    Unknown(Box<str>),
}

impl Directive {
    pub fn new(directive: &'static str) -> Self {
        Self::Standard(directive)
    }

    pub fn unknown(unknown: Box<str>) -> Self {
        Self::Unknown(unknown)
    }

    pub fn len_with_prefix(&self) -> usize {
        1 + match self {
            Directive::Standard(s) => s.len(),
            Directive::Unknown(s) => s.len(),
        }
    }
}

impl AsRef<str> for Directive {
    fn as_ref(&self) -> &str {
        match self {
            Directive::Standard(s) => s,
            Directive::Unknown(s) => s,
        }
    }
}

impl Display for Directive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "@{}", self.as_ref())
    }
}

#[derive(Clone, PartialEq)]
pub struct Punct([u8; 4]);

impl Punct {
    pub const fn new(s: &'static str) -> Self {
        let str_bytes = s.as_bytes();
        let mut chars: [u8; 4] = *b"\0\0\0\0";
        let mut i = 0;

        while i < str_bytes.len() && i < 4 {
            let c = str_bytes[i];

            if c >= 128 {
                break;
            }

            chars[i] = c;
            i += 1;
        }

        Self(chars)
    }

    pub fn len(&self) -> usize {
        self.0.iter().position(|c| *c == b'\0').unwrap_or(4)
    }

    #[inline]
    pub const fn is(&self, possible: &'static str) -> bool {
        self.const_eq(Punct::new(possible))
    }

    #[inline]
    pub const fn const_eq(&self, other: Punct) -> bool {
        u32::from_ne_bytes(self.0) == u32::from_ne_bytes(other.0)
    }

    #[inline]
    pub const fn is_any(&self, possible: &[&'static str]) -> bool {
        let mut i = 0;

        while i < possible.len() {
            if self.const_eq(Punct::new(possible[i])) {
                return true;
            }
            i += 1;
        }

        false
    }

    pub fn as_str(&self) -> &str {
        str::from_utf8(&self.0[..self.len()]).unwrap()
    }
}

impl Display for Punct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Debug for Punct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Punct").field(&self.as_str()).finish()
    }
}

#[derive(Clone, Debug, PartialEq, IsVariant, Unwrap)]
pub enum TokenKind {
    EndOfFile,
    Error(char),
    ColumnSpacing(ColumnSpacingAtom),
    Newline,
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
            Self::Error(message) => write!(f, "'lex error - {}'", message),
            Self::ColumnSpacing(atom) => write!(f, "column spacing {:?}", atom),
            Self::Newline => f.write_str("'newline'"),
            Self::Identifier(name) => write!(f, "(identifier) '{}'", name),
            Self::Polymorph(name) => write!(f, "'${}'", name),
            Self::Grouping(c) => write!(f, "'{}'", c),
            Self::String { .. } => f.write_str("'string'"),
            Self::MissingStringTermination => f.write_str("'missing string termination'"),
            Self::Integer { .. } => f.write_str("'integer'"),
            Self::Float { .. } => f.write_str("'float'"),
            Self::Directive(directive) => write!(f, "'{}'", directive),
            Self::Punct(punct) => write!(f, "'{}'", punct),
            Self::Label(name) => write!(f, "goto label '@{}@'", name),
        }
    }
}

const ASSIGNMENT_OPERATORS: &[&'static str] = &[
    "=", ":=", "+=", "-=", "*=", "/=", "%=", "&=", "^=", "|=", "<<=", "<<<=", ">>=", ">>>=",
];

const NON_ASSIGNMENT_OPERATORS: &[&'static str] = &[
    ",", ".", ":", "::", "++", "--", "!", "~", "*", "/", "%", "+", "-", "<<", "<<<", ">>", ">>>",
    "<", "<=", ">", ">=", "==", "!=", "&", "^", "|", "&&", "||",
];

pub const ALL_DIRECTIVES: &[&'static str] = &["fn", "type", "struct", "enum", "record"];

pub const ALL_PUNCT_GROUPS: &[&'static [&'static str]] =
    &[ASSIGNMENT_OPERATORS, NON_ASSIGNMENT_OPERATORS];

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
            | Self::Newline
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
            TokenKind::Newline => 1,
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
