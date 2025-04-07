mod encoding;
mod punctuator;

use derive_more::IsVariant;
pub use encoding::Encoding;
use inflow::InflowEnd;
pub use punctuator::Punctuator;
use source_files::Source;
use std::fmt::Display;

#[derive(Clone, Debug)]
pub struct PreToken {
    pub kind: PreTokenKind,
    pub source: Source,
}

impl PreToken {
    pub fn new(kind: PreTokenKind, source: Source) -> Self {
        Self { kind, source }
    }

    /// Converts token into a form that isn't affected by the preprocessor
    pub fn protect(self) -> Self {
        let PreToken { kind, source } = self;

        match kind {
            PreTokenKind::Identifier(name) => PreTokenKind::ProtectedIdentifier(name).at(source),
            _ => kind.at(source),
        }
    }
}

impl Display for PreToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.kind.fmt(f)
    }
}

#[derive(Clone, Debug, Hash, IsVariant)]
pub enum PreTokenKind {
    EndOfSequence,
    HeaderName(String),
    Identifier(String),
    ProtectedIdentifier(String),
    Number(String),
    CharacterConstant(Encoding, String),
    StringLiteral(Encoding, String),
    Punctuator(Punctuator),
    UniversalCharacterName(char), // e.g. '\u1F3E'
    IsDefined(String),            // e.g. `defined NAME` or `defined(NAME)`
    Other(char),
    Placeholder, // a non-lexical token, has no textual representation (used for '##' concats with empty)
}

impl PreToken {
    pub fn is_hash(&self) -> bool {
        matches!(self.kind, PreTokenKind::Punctuator(Punctuator::Hash))
    }

    pub fn identifier(&self) -> Option<&str> {
        match &self.kind {
            PreTokenKind::Identifier(identifier) => Some(identifier),
            _ => None,
        }
    }

    pub fn is_identifier(&self, content: &str) -> bool {
        matches!(&self.kind, PreTokenKind::Identifier(identifier) if content == identifier)
    }

    pub fn get_identifier(&self) -> Option<&str> {
        match &self.kind {
            PreTokenKind::Identifier(identifier) => Some(identifier),
            _ => None,
        }
    }

    pub fn is_open_paren_disregard_whitespace(&self) -> bool {
        matches!(
            self.kind,
            PreTokenKind::Punctuator(Punctuator::OpenParen { .. })
        )
    }
}

impl Display for PreTokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PreTokenKind::EndOfSequence => write!(f, "<end of sequence>"),
            PreTokenKind::HeaderName(name) => write!(f, "<{}>", name),
            PreTokenKind::Identifier(identifier) => f.write_str(identifier),
            PreTokenKind::ProtectedIdentifier(identifier) => {
                write!(f, "<protected> {}", identifier)
            }
            PreTokenKind::Number(number) => f.write_str(number),
            PreTokenKind::CharacterConstant(_, content) => write!(f, "'{}'", escape(content, '\'')),
            PreTokenKind::StringLiteral(_, content) => write!(f, "\"{}\"", escape(content, '"')),
            PreTokenKind::Punctuator(punctuator) => punctuator.fmt(f),
            PreTokenKind::UniversalCharacterName(_) => Ok(()),
            PreTokenKind::IsDefined(name) => write!(f, "defined({})", name),
            PreTokenKind::Other(c) => write!(f, "{}", c),
            PreTokenKind::Placeholder => Ok(()),
        }
    }
}

impl PreTokenKind {
    pub fn at(self, source: Source) -> PreToken {
        PreToken { kind: self, source }
    }

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
            Punctuator::Ternary => 2,
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

impl InflowEnd for PreToken {
    fn is_inflow_end(&self) -> bool {
        self.kind.is_inflow_end()
    }
}

impl InflowEnd for PreTokenKind {
    fn is_inflow_end(&self) -> bool {
        matches!(self, PreTokenKind::EndOfSequence)
    }
}
