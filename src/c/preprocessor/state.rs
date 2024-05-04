use super::token::PreTokenKind;

#[derive(Clone, Debug)]
pub enum State {
    Idle,
    Number(String),
    MultiLineComment,
    Identifier(String),
    CharacterConstant(String),
    StringLiteral(String),
}

impl State {
    pub fn finalize(&mut self) -> Option<PreTokenKind> {
        match std::mem::replace(self, State::Idle) {
            Self::Idle => None,
            Self::Number(value) => Some(PreTokenKind::Number(value)),
            Self::MultiLineComment => None,
            Self::Identifier(value) => Some(PreTokenKind::Identifier(value)),
            Self::CharacterConstant(value) => Some(PreTokenKind::CharacterConstant(value)),
            Self::StringLiteral(value) => Some(PreTokenKind::StringLiteral(value)),
        }
    }
}
