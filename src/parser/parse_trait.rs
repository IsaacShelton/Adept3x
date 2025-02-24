use super::{
    annotation::{Annotation, AnnotationKind},
    error::ParseError,
    Parser,
};
use crate::{
    ast::{Params, Privacy, Trait, TraitFunc},
    inflow::Inflow,
    token::{Token, TokenKind},
};

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn parse_trait(&mut self, annotations: Vec<Annotation>) -> Result<Trait, ParseError> {
        let source = self.source_here();
        self.input.advance();

        let name = self.parse_identifier(Some("for trait name after 'trait' keyword"))?;
        self.ignore_newlines();

        let mut privacy = Privacy::Protected;

        for annotation in annotations {
            match annotation.kind {
                AnnotationKind::Public => privacy = Privacy::Public,
                AnnotationKind::Private => privacy = Privacy::Private,
                _ => return Err(self.unexpected_annotation(&annotation, Some("for trait"))),
            }
        }

        self.ignore_newlines();

        let params = self.parse_type_params()?;
        self.parse_token(TokenKind::OpenCurly, Some("to begin trait body"))?;
        self.ignore_newlines();

        let mut funcs = vec![];

        while !self.input.peek_is_or_eof(TokenKind::CloseCurly) {
            funcs.push(self.parse_trait_method()?);
            self.ignore_newlines();
        }

        self.parse_token(TokenKind::CloseCurly, Some("to end trait body"))?;

        Ok(Trait {
            name,
            params,
            source,
            privacy,
            funcs,
        })
    }

    fn parse_trait_method(&mut self) -> Result<TraitFunc, ParseError> {
        let source = self.input.peek().source;

        if !self.input.eat(TokenKind::FuncKeyword) {
            return Err(ParseError::expected(
                "'func' keyword",
                Some("to begin trait method"),
                self.input.peek(),
            ));
        }

        let name = self.parse_identifier(Some("after 'func' keyword"))?;
        self.ignore_newlines();

        let parameters = if self.input.peek_is(TokenKind::OpenParen) {
            self.parse_func_params()?
        } else {
            Params::default()
        };

        self.ignore_newlines();

        let return_type = self.parse_type(Some("return"), Some("for trait method"))?;

        Ok(TraitFunc {
            name,
            params: parameters,
            return_type,
            source,
        })
    }
}
