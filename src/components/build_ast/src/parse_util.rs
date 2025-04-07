use super::{
    Parser,
    error::{ParseError, ParseErrorKind},
};
use ast::Name;
use inflow::Inflow;
use source_files::{Source, Sourced};
use std::borrow::Borrow;
use token::{Token, TokenKind};

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
        Ok(self
            .parse_identifier_keep_location(for_reason)?
            .inner()
            .into())
    }

    pub fn parse_identifier_keep_location(
        &mut self,
        for_reason: Option<impl ToString>,
    ) -> Result<Sourced<String>, ParseError> {
        let token = self.input.advance();

        if let TokenKind::Identifier(identifier) = &token.kind {
            Ok(Sourced::new(identifier.into(), token.source))
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
