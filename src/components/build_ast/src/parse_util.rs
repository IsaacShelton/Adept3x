use super::{
    Parser,
    error::{ParseError, ParseErrorKind},
};
use ast::NamePath;
use infinite_iterator::InfinitePeekable;
use optional_string::OptionalString;
use smallvec::SmallVec;
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

    pub fn parse_name_path(
        &mut self,
        for_reason: impl OptionalString,
    ) -> Result<NamePath, ParseError> {
        let mut segments = SmallVec::new();

        segments.push(
            self.input
                .eat_identifier()
                .ok_or_else(|| {
                    ParseError::expected("identifier", for_reason, self.input.advance())
                })?
                .into(),
        );

        while self.input.eat(TokenKind::StaticMember) {
            let Some(segment) = self.input.eat_identifier() else {
                return Err(ParseError::other(
                    "Expected identifier after '::'",
                    self.input.here(),
                ));
            };

            segments.push(segment.into());
        }

        Ok(NamePath::new(segments))
    }

    pub fn ignore_newlines(&mut self) {
        while self.input.peek().kind.is_newline() {
            self.input.advance();
        }
    }
}

pub fn into_plain_name(name_path: NamePath, source: Source) -> Result<Box<str>, ParseError> {
    name_path
        .into_plain()
        .ok_or_else(|| ParseErrorKind::NamespaceNotAllowedHere.at(source))
}
