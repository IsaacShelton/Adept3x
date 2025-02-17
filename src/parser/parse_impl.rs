use super::{
    annotation::{Annotation, AnnotationKind},
    error::{ParseError, ParseErrorKind},
    Parser,
};
use crate::{
    ast::{Impl, Privacy, TypeParams},
    inflow::Inflow,
    token::{Token, TokenKind},
};

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn parse_impl(&mut self, annotations: Vec<Annotation>) -> Result<Impl, ParseError> {
        let source = self.input.peek().source;
        self.input.advance().kind.unwrap_impl_keyword();

        let mut privacy = Privacy::Private;

        for annotation in annotations {
            match annotation.kind {
                AnnotationKind::Public => privacy = Privacy::Public,
                _ => {
                    return Err(self.unexpected_annotation(&annotation, Some("for implementation")))
                }
            }
        }

        let target = self.parse_type(Some("trait"), None::<&str>)?;

        let name = self.input.eat_identifier();
        let params = TypeParams::from(self.parse_type_params()?);
        let mut body = vec![];

        if !self.input.eat(TokenKind::OpenCurly) {
            return Err(ParseError::expected(
                "'{'",
                Some("to begin implementation body"),
                self.input.peek(),
            ));
        }

        self.input.ignore_newlines();

        while !self.input.peek_is_or_eof(TokenKind::CloseCurly) {
            // Ignore preceeding newlines
            self.ignore_newlines();

            // Parse annotations
            let mut annotations = vec![];
            while self.input.peek().is_hash() {
                annotations.extend(self.parse_annotation()?);
                self.ignore_newlines();
            }

            body.push(self.parse_func(annotations)?);
            self.input.ignore_newlines();
        }

        if !self.input.eat(TokenKind::CloseCurly) {
            return Err(ParseErrorKind::Expected {
                expected: TokenKind::CloseCurly.to_string(),
                for_reason: Some("to close implementation body".into()),
                got: self.input.peek().to_string(),
            }
            .at(self.input.peek().source));
        }

        Ok(Impl {
            name,
            params,
            target,
            source,
            privacy,
            body,
        })
    }
}
