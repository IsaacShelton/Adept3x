use super::{
    annotation::{Annotation, AnnotationKind},
    error::ParseError,
    Parser,
};
use crate::{
    ast::{Function, Parameters, Privacy, TypeKind},
    inflow::Inflow,
    name::Name,
    token::{Token, TokenKind},
};

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn parse_function(&mut self, annotations: Vec<Annotation>) -> Result<Function, ParseError> {
        // func functionName {
        //   ^

        if !self.input.peek().is_func_keyword() {
            return Err(ParseError::expected(
                "function",
                None::<&str>,
                self.input.peek(),
            ));
        }

        let source = self.input.advance().source;

        let mut is_foreign = false;
        let mut abide_abi = false;
        let mut privacy = Privacy::Private;
        let mut contextual_parameters = vec![];

        for annotation in annotations {
            match annotation.kind {
                AnnotationKind::Foreign => is_foreign = true,
                AnnotationKind::AbideAbi => abide_abi = true,
                AnnotationKind::Public => privacy = Privacy::Public,
                AnnotationKind::Given(given) => {
                    contextual_parameters.push(given);
                }
                _ => return Err(self.unexpected_annotation(&annotation, Some("for function"))),
            }
        }

        // abide_abi is implied for all foreign functions
        if is_foreign {
            abide_abi = true;
        }

        let name = self.parse_identifier(Some("after 'func' keyword"))?;
        self.ignore_newlines();

        let parameters = if self.input.peek_is(TokenKind::OpenParen) {
            self.parse_function_parameters()?
        } else {
            Parameters::default()
        };

        self.ignore_newlines();

        let return_type = if self.input.peek_is(TokenKind::OpenCurly) {
            TypeKind::Void.at(self.source_here())
        } else {
            self.parse_type(Some("return "), Some("for function"))?
        };

        let stmts = (!is_foreign)
            .then(|| self.parse_block("function"))
            .transpose()?
            .unwrap_or_default();

        Ok(Function {
            name: Name::plain(name),
            parameters,
            return_type,
            stmts,
            is_foreign,
            source,
            abide_abi,
            tag: None,
            privacy,
        })
    }
}
