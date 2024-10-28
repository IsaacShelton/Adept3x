use super::{
    annotation::{Annotation, AnnotationKind},
    error::ParseError,
    Parser,
};
use crate::{
    ast::{Privacy, Trait},
    inflow::Inflow,
    token::{Token, TokenKind},
};

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn parse_trait(&mut self, annotations: Vec<Annotation>) -> Result<Trait, ParseError> {
        let source = self.source_here();
        self.input.advance();

        let name = self.parse_identifier(Some("for trait name after 'trait' keyword"))?;
        self.ignore_newlines();

        let mut privacy = Privacy::Private;

        for annotation in annotations {
            match annotation.kind {
                AnnotationKind::Public => privacy = Privacy::Public,
                _ => return Err(self.unexpected_annotation(&annotation, Some("for trait"))),
            }
        }

        self.ignore_newlines();
        self.parse_token(TokenKind::OpenCurly, Some("to begin trait body"))?;

        while !self.input.peek_is_or_eof(TokenKind::CloseCurly) {
            todo!("parse trait body");
        }

        self.parse_token(TokenKind::CloseCurly, Some("to end trait body"))?;

        Ok(Trait {
            name,
            source,
            privacy,
        })
    }
}
