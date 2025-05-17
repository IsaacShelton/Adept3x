use super::{
    Parser,
    annotation::Annotation,
    error::{ParseError, ParseErrorKind},
};
use infinite_iterator::InfinitePeekable;
use optional_string::OptionalString;
use source_files::Source;
use token::{Token, TokenKind};

impl<I: InfinitePeekable<Token>> Parser<'_, I> {
    pub fn unexpected_token_is_next(&mut self) -> ParseError {
        let unexpected = self.input.advance();
        self.unexpected_token(&unexpected)
    }

    pub fn unexpected_token(&self, token: &Token) -> ParseError {
        match &token.kind {
            TokenKind::Error(message) => ParseErrorKind::Lexical {
                message: message.into(),
            },
            _ => {
                let unexpected = token.to_string();
                ParseErrorKind::UnexpectedToken { unexpected }
            }
        }
        .at(token.source)
    }

    pub fn expected_top_level_construct(&self, token: &Token) -> ParseError {
        ParseErrorKind::ExpectedTopLevelConstruct.at(token.source)
    }

    pub fn unexpected_annotation(
        &self,
        annotation: &Annotation,
        for_reason: impl OptionalString,
    ) -> ParseError {
        self.unexpected_annotation_ex(annotation.kind.to_string(), annotation.source, for_reason)
    }

    pub fn unexpected_annotation_ex(
        &self,
        name: String,
        source: Source,
        for_reason: impl OptionalString,
    ) -> ParseError {
        ParseErrorKind::UnexpectedAnnotation {
            name,
            for_reason: for_reason.to_option_string(),
        }
        .at(source)
    }
}
