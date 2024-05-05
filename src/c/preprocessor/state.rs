use super::token::{Encoding, PreTokenKind};

#[derive(Clone, Debug)]
pub enum State {
    Idle,
    Number(String),
    MultiLineComment,
    Identifier(String),
    CharacterConstant(Encoding, String),
    StringLiteral(Encoding, String),
}

impl State {
    pub fn string(encoding: Encoding) -> Self {
        Self::StringLiteral(encoding, "".into())
    }

    pub fn character(encoding: Encoding) -> Self {
        Self::CharacterConstant(encoding, "".into())
    }

    pub fn finalize(&mut self) -> Option<PreTokenKind> {
        match std::mem::replace(self, State::Idle) {
            Self::Idle => None,
            Self::Number(value) => Some(PreTokenKind::Number(value)),
            Self::MultiLineComment => None,
            Self::Identifier(value) => Some(PreTokenKind::Identifier(value)),
            Self::CharacterConstant(encoding, value) => Some(PreTokenKind::CharacterConstant(encoding, value)),
            Self::StringLiteral(encoding, value) => Some(PreTokenKind::StringLiteral(encoding, value)),
        }
    }
}
