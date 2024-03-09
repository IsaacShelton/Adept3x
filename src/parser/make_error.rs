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
        let info = self.input.next();
        self.unexpected_token(info)
    }

    pub fn unexpected_token(&self, info: Option<TokenInfo>) -> ParseError {
        let (unexpected, location) = info
            .map(|info| (Some(info.token.to_string()), info.location))
            .unwrap_or_else(|| (None, self.input.previous_location()));

        ParseError {
            filename: Some(self.input.filename().to_string()),
            location: Some(location),
            info: ErrorInfo::UnexpectedToken { unexpected },
        }
    }

    pub fn expected_token(
        &self,
        expected: impl ToString,
        for_reason: Option<impl ToString>,
        info: Option<TokenInfo>,
    ) -> ParseError {
        let (got, location) = info
            .map(|info| (Some(info.token.to_string()), info.location))
            .unwrap_or_else(|| (None, self.input.previous_location()));

        ParseError {
            filename: Some(self.input.filename().to_string()),
            location: Some(location),
            info: ErrorInfo::Expected {
                expected: expected.to_string(),
                for_reason: for_reason.map(|reason| reason.to_string()),
                got,
            },
        }
    }

    pub fn expected_top_level_construct(&self, info: Option<TokenInfo>) -> ParseError {
        let location = info
            .map(|info| info.location)
            .unwrap_or_else(|| self.input.previous_location());

        ParseError {
            filename: Some(self.input.filename().to_string()),
            location: Some(location),
            info: ErrorInfo::ExpectedTopLevelConstruct,
        }
    }
}
