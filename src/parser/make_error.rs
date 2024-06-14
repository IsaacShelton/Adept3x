use super::{
    error::{ParseError, ParseErrorKind},
    Parser,
};
use crate::{
    line_column::Location,
    token::{Token, TokenKind},
};

impl<I> Parser<'_, I>
where
    I: Iterator<Item = Token>,
{
    pub fn unexpected_token_is_next(&mut self) -> ParseError {
        let unexpected = self.input.advance();
        self.unexpected_token(&unexpected)
    }

    pub fn unexpected_token(&self, token: &Token) -> ParseError {
        ParseError {
            kind: match &token.kind {
                TokenKind::Error(message) => ParseErrorKind::Lexical {
                    message: message.into(),
                },
                _ => {
                    let unexpected = token.to_string();
                    ParseErrorKind::UnexpectedToken { unexpected }
                }
            },
            source: self.source(token.location),
        }
    }

    pub fn expected_token(
        &self,
        expected: impl ToString,
        for_reason: Option<impl ToString>,
        token: Token,
    ) -> ParseError {
        ParseError {
            kind: match &token.kind {
                TokenKind::Error(message) => ParseErrorKind::Lexical {
                    message: message.into(),
                },
                _ => ParseErrorKind::Expected {
                    expected: expected.to_string(),
                    for_reason: for_reason.map(|reason| reason.to_string()),
                    got: token.kind.to_string(),
                },
            },
            source: self.source(token.location),
        }
    }

    pub fn expected_top_level_construct(&self, token: &Token) -> ParseError {
        ParseError {
            kind: ParseErrorKind::ExpectedTopLevelConstruct,
            source: self.source(token.location),
        }
    }

    pub fn unexpected_annotation(
        &self,
        name: String,
        location: Location,
        for_reason: Option<impl ToString>,
    ) -> ParseError {
        ParseError {
            kind: ParseErrorKind::UnexpectedAnnotation {
                name,
                for_reason: for_reason.map(|reason| reason.to_string()),
            },
            source: self.source(location),
        }
    }
}
