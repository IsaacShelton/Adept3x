use super::{
    annotation::Annotation,
    error::{ParseError, ParseErrorKind},
    Parser,
};
use crate::{
    inflow::Inflow,
    source_files::Source,
    token::{Token, TokenKind},
};

impl<I: Inflow<Token>> Parser<'_, I> {
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
        for_reason: Option<impl ToString>,
    ) -> ParseError {
        self.unexpected_annotation_ex(annotation.kind.to_string(), annotation.source, for_reason)
    }

    pub fn unexpected_annotation_ex(
        &self,
        name: String,
        source: Source,
        for_reason: Option<impl ToString>,
    ) -> ParseError {
        ParseErrorKind::UnexpectedAnnotation {
            name,
            for_reason: for_reason.map(|reason| reason.to_string()),
        }
        .at(source)
    }
}
