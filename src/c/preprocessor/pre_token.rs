use crate::c::encoding::Encoding;
use std::fmt::Display;

pub use crate::c::punctuator::Punctuator;

#[derive(Clone, Debug, Hash)]
pub struct PreToken {
    pub kind: PreTokenKind,
}

impl PreToken {
    pub fn new(kind: PreTokenKind) -> Self {
        Self { kind }
    }
}

impl Display for PreToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.kind.fmt(f)
    }
}

#[derive(Clone, Debug, Hash)]
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

impl Display for PreTokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PreTokenKind::HeaderName(name) => write!(f, "<{}>", name),
            PreTokenKind::Identifier(identifier) => f.write_str(identifier),
            PreTokenKind::Number(number) => f.write_str(number),
            PreTokenKind::CharacterConstant(_, content) => write!(f, "'{}'", escape(content, '\'')),
            PreTokenKind::StringLiteral(_, content) => write!(f, "\"{}\"", escape(content, '"')),
            PreTokenKind::Punctuator(punctuator) => punctuator.fmt(f),
            PreTokenKind::UniversalCharacterName(_) => Ok(()),
            PreTokenKind::Other(c) => write!(f, "{}", c),
        }
    }
}

impl PreTokenKind {
    pub fn precedence(&self) -> usize {
        let punctuator = match self {
            Self::Punctuator(punctuator) => punctuator,
            _ => return 0,
        };

        match punctuator {
            Punctuator::Increment => 15,
            Punctuator::Decrement => 15,
            Punctuator::Not => 14,
            Punctuator::BitComplement => 14,
            Punctuator::Multiply => 12,
            Punctuator::Divide => 12,
            Punctuator::Modulus => 12,
            Punctuator::Add => 11,
            Punctuator::Subtract => 11,
            Punctuator::LeftShift => 10,
            Punctuator::RightShift => 10,
            Punctuator::LessThan => 9,
            Punctuator::GreaterThan => 9,
            Punctuator::LessThanEq => 9,
            Punctuator::GreaterThanEq => 9,
            Punctuator::DoubleEquals => 8,
            Punctuator::NotEquals => 8,
            Punctuator::Ampersand => 7,
            Punctuator::BitXor => 6,
            Punctuator::BitOr => 5,
            Punctuator::LogicalAnd => 4,
            Punctuator::LogicalOr => 3,
            Punctuator::MultiplyAssign
            | Punctuator::DivideAssign
            | Punctuator::ModulusAssign
            | Punctuator::AddAssign
            | Punctuator::SubtractAssign
            | Punctuator::LeftShiftAssign
            | Punctuator::RightShiftAssign
            | Punctuator::BitAndAssign
            | Punctuator::BitXorAssign
            | Punctuator::BitOrAssign => 1,
            _ => 0,
        }
    }
}

fn escape(content: &str, around: char) -> String {
    let mut result = String::with_capacity(content.len() + 16);

    for c in content.chars() {
        if c == around {
            result.push('\\');
            result.push(around);
            continue;
        }

        match c {
            '\\' => result.push_str("\\\\"),
            '\u{07}' => result.push_str("\\a"),
            '\u{08}' => result.push_str("\\b"),
            '\u{0C}' => result.push_str("\\f"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            '\u{0B}' => result.push_str("\\v"),
            c => result.push(c),
        }
    }

    result
}