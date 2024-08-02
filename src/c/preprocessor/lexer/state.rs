use crate::{
    c::{
        encoding::Encoding,
        preprocessor::{pre_token::PreTokenKind, PreToken},
    },
    source_files::Source,
};

#[derive(Clone, Debug, Default)]
pub enum State {
    #[default]
    Idle,
    Number(String, Source),
    MultiLineComment(Source),
    Identifier(String, Source),
    CharacterConstant(Encoding, String, Source),
    StringLiteral(Encoding, String, Source),
    HeaderName(String, Source),
}

impl State {
    pub fn string(encoding: Encoding, source: Source) -> Self {
        Self::StringLiteral(encoding, "".into(), source)
    }

    pub fn character(encoding: Encoding, source: Source) -> Self {
        Self::CharacterConstant(encoding, "".into(), source)
    }

    pub fn finalize(&mut self) -> Option<PreToken> {
        match std::mem::replace(self, State::Idle) {
            Self::Idle => None,
            Self::Number(value, source) => Some(PreTokenKind::Number(value).at(source)),
            Self::MultiLineComment(_) => None,
            Self::Identifier(value, source) => Some(PreTokenKind::Identifier(value).at(source)),
            Self::CharacterConstant(encoding, value, source) => {
                Some(PreTokenKind::CharacterConstant(encoding, value).at(source))
            }
            Self::StringLiteral(encoding, value, source) => {
                Some(PreTokenKind::StringLiteral(encoding, value).at(source))
            }
            Self::HeaderName(value, source) => Some(PreTokenKind::HeaderName(value).at(source)),
        }
    }
}
