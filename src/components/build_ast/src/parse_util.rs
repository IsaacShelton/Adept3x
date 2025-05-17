use super::{
    Parser,
    error::{ParseError, ParseErrorKind},
};
use ast::Name;
use infinite_iterator::InfinitePeekable;
use optional_string::OptionalString;
use source_files::{Source, Sourced};
use token::{Token, TokenKind};

impl<'a, I: InfinitePeekable<Token>> Parser<'a, I> {
    pub fn parse_identifier(
        &mut self,
        for_reason: impl OptionalString,
    ) -> Result<String, ParseError> {
        Ok(self
            .parse_identifier_keep_location(for_reason)?
            .inner()
            .into())
    }

    pub fn parse_identifier_keep_location(
        &mut self,
        for_reason: impl OptionalString,
    ) -> Result<Sourced<String>, ParseError> {
        let token = self.input.advance();

        if let TokenKind::Identifier(identifier) = &token.kind {
            Ok(Sourced::new(identifier.into(), token.source))
        } else {
            Err(ParseError::expected("identifier", for_reason, token))
        }
    }

    pub fn parse_name(&mut self, for_reason: impl OptionalString) -> Result<Name, ParseError> {
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
