use super::{
    annotation::Annotation,
    error::{ParseError, ParseErrorKind},
    Parser,
};
use crate::{
    ast::Impl,
    inflow::Inflow,
    token::{Token, TokenKind},
};

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn parse_impl(&mut self, annotations: Vec<Annotation>) -> Result<Impl, ParseError> {
        let source = self.input.peek().source;
        self.input.advance().kind.unwrap_impl_keyword();

        for annotation in annotations {
            match annotation.kind {
                _ => return Err(self.unexpected_annotation(&annotation, Some("for impl"))),
            }
        }

        let target_trait = self.parse_type(None::<&str>, Some("trait"))?;

        if !self.input.eat(TokenKind::ForKeyword) {
            return Err(ParseErrorKind::Expected {
                expected: TokenKind::ForKeyword.to_string(),
                for_reason: Some("after trait to implement".into()),
                got: self.input.peek().to_string(),
            }
            .at(self.input.peek().source));
        }

        let for_type = self.parse_type(None::<&str>, Some("impl target"))?;

        let mut body = vec![];

        if self.input.eat(TokenKind::OpenCurly) {
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

                body.push(self.parse_function(annotations)?);
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
        }

        Ok(Impl {
            for_type,
            target_trait,
            source,
            body,
        })
    }
}
