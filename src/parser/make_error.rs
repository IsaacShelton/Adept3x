use super::{
    error::{ParseError, ParseErrorKind},
    Parser,
};
use crate::{line_column::Location, token::Token};

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
            filename: Some(self.input.filename().to_string()),
            location: Some(token.location),
            kind: ParseErrorKind::UnexpectedToken {
                unexpected: token.kind.to_string(),
            },
        }
    }

    pub fn expected_token(
        &self,
        expected: impl ToString,
        for_reason: Option<impl ToString>,
        token: Token,
    ) -> ParseError {
        ParseError {
            filename: Some(self.input.filename().to_string()),
            location: Some(token.location),
            kind: ParseErrorKind::Expected {
                expected: expected.to_string(),
                for_reason: for_reason.map(|reason| reason.to_string()),
                got: token.kind.to_string(),
            },
        }
    }

    pub fn expected_top_level_construct(&self, token: &Token) -> ParseError {
        ParseError {
            filename: Some(self.input.filename().to_string()),
            location: Some(token.location),
            kind: ParseErrorKind::ExpectedTopLevelConstruct,
        }
    }

    pub fn unexpected_annotation(
        &self,
        name: String,
        location: Location,
        for_reason: Option<impl ToString>,
    ) -> ParseError {
        ParseError {
            filename: Some(self.input.filename().to_string()),
            location: Some(location),
            kind: ParseErrorKind::UnexpectedAnnotation {
                name,
                for_reason: for_reason.map(|reason| reason.to_string()),
            },
        }
    }
}
