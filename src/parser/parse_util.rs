use super::{
    error::{ParseError, ParseErrorKind},
    Parser,
};
use crate::{
    inflow::Inflow,
    name::Name,
    source_files::Source,
    token::{Token, TokenKind},
};
use std::borrow::Borrow;

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn parse_token(
        &mut self,
        expected_token: impl Borrow<TokenKind>,
        for_reason: Option<impl ToString>,
    ) -> Result<Source, ParseError> {
        let token = self.input.advance();
        let expected_token = expected_token.borrow();

        if token.kind == *expected_token {
            return Ok(token.source);
        }

        Err(ParseError::expected(expected_token, for_reason, token))
    }

    pub fn parse_identifier(
        &mut self,
        for_reason: Option<impl ToString>,
    ) -> Result<String, ParseError> {
        Ok(self.parse_identifier_keep_location(for_reason)?.0)
    }

    pub fn parse_identifier_keep_location(
        &mut self,
        for_reason: Option<impl ToString>,
    ) -> Result<(String, Source), ParseError> {
        let token = self.input.advance();

        if let TokenKind::Identifier(identifier) = &token.kind {
            Ok((identifier.into(), token.source))
        } else {
            Err(ParseError::expected("identifier", for_reason, token))
        }
    }

    pub fn parse_name(&mut self, for_reason: Option<impl ToString>) -> Result<Name, ParseError> {
        let token = self.input.advance();

        match token.kind {
            TokenKind::NamespacedIdentifier(name) => Ok(name),
            TokenKind::Identifier(basename) => Ok(Name::plain(basename)),
            _ => Err(ParseError::expected("identifier", for_reason, token)),
        }
    }

    pub fn ignore_newlines(&mut self) {
        while self.input.peek().kind.is_newline() {
            self.input.advance();
        }
    }
}

pub fn into_plain_name(name: Name, source: Source) -> Result<String, ParseError> {
    name.into_plain()
        .ok_or_else(|| ParseErrorKind::NamespaceNotAllowedHere.at(source))
}
