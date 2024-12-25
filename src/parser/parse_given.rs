use super::{
    annotation::Annotation,
    error::{ParseError, ParseErrorKind},
    Parser,
};
use crate::{
    ast::Given,
    inflow::Inflow,
    token::{Token, TokenKind},
};

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn parse_given(&mut self, annotations: Vec<Annotation>) -> Result<Given, ParseError> {
        let source = self.input.peek().source;
        self.input.advance().kind.unwrap_given_keyword();

        for annotation in annotations {
            match annotation.kind {
                _ => {
                    return Err(self.unexpected_annotation(&annotation, Some("for implementation")))
                }
            }
        }

        let name = self.parse_optional_name();
        let target = self.parse_type(None::<&str>, Some("trait"))?;

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

        Ok(Given {
            name,
            target,
            source,
            body,
        })
    }
}
