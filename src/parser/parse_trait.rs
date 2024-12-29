use super::{
    annotation::{Annotation, AnnotationKind},
    error::{ParseError, ParseErrorKind},
    Parser,
};
use crate::{
    ast::{Params, Privacy, Trait, TraitFunc},
    inflow::Inflow,
    token::{Token, TokenKind},
};
use itertools::Itertools;

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

        let parameters = self.parse_type_parameters()?;
        self.parse_token(TokenKind::OpenCurly, Some("to begin trait body"))?;
        self.ignore_newlines();

        if parameters
            .values()
            .any(|constraints| !constraints.constraints.is_empty())
        {
            return Err(ParseErrorKind::Other {
                message: "Constraints not supported on traits yet".into(),
            }
            .at(source));
        }

        let parameters = parameters.into_keys().collect_vec();

        let mut methods = vec![];

        while !self.input.peek_is_or_eof(TokenKind::CloseCurly) {
            methods.push(self.parse_trait_method()?);
            self.ignore_newlines();
        }

        self.parse_token(TokenKind::CloseCurly, Some("to end trait body"))?;

        Ok(Trait {
            name,
            parameters,
            source,
            privacy,
            funcs: methods,
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

        let return_type = self.parse_type(Some("return "), Some("for trait method"))?;

        Ok(TraitFunc {
            name,
            params: parameters,
            return_type,
            source,
        })
    }
}
