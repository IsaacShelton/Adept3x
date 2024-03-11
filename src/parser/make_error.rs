use super::{
    error::{ErrorInfo, ParseError},
    Parser,
};
use crate::token::TokenInfo;

impl<I> Parser<I>
where
    I: Iterator<Item = TokenInfo>,
{
    pub fn unexpected_token_is_next(&mut self) -> ParseError {
        let unexpected = self.input.advance();
        self.unexpected_token(&unexpected)
    }

    pub fn unexpected_token(&self, info: &TokenInfo) -> ParseError {
        ParseError {
            filename: Some(self.input.filename().to_string()),
            location: Some(info.location),
            info: ErrorInfo::UnexpectedToken { unexpected: info.token.to_string() },
        }
    }

    pub fn expected_token(
        &self,
        expected: impl ToString,
        for_reason: Option<impl ToString>,
        info: TokenInfo,
    ) -> ParseError {
        ParseError {
            filename: Some(self.input.filename().to_string()),
            location: Some(info.location),
            info: ErrorInfo::Expected {
                expected: expected.to_string(),
                for_reason: for_reason.map(|reason| reason.to_string()),
                got: info.token.to_string(),
            },
        }
    }

    pub fn expected_top_level_construct(&self, info: &TokenInfo) -> ParseError {
        ParseError {
            filename: Some(self.input.filename().to_string()),
            location: Some(info.location),
            info: ErrorInfo::ExpectedTopLevelConstruct,
        }
    }
}
