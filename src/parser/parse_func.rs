use super::{
    annotation::{Annotation, AnnotationKind},
    error::ParseError,
    Parser,
};
use crate::{
    ast::{Func, FuncHead, Privacy, TypeKind},
    inflow::Inflow,
    token::{Token, TokenKind},
};

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn parse_func(&mut self, annotations: Vec<Annotation>) -> Result<Func, ParseError> {
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
        let mut givens = vec![];

        for annotation in annotations {
            match annotation.kind {
                AnnotationKind::Foreign => is_foreign = true,
                AnnotationKind::AbideAbi => abide_abi = true,
                AnnotationKind::Public => privacy = Privacy::Public,
                AnnotationKind::Given(given) => {
                    givens.push(given);
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

        let params = self
            .input
            .peek_is(TokenKind::OpenParen)
            .then(|| self.parse_func_params())
            .transpose()?
            .unwrap_or_default();

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

        Ok(Func {
            head: FuncHead {
                name,
                givens,
                params,
                return_type,
                is_foreign,
                source,
                abide_abi,
                tag: None,
                privacy,
            },
            stmts,
        })
    }
}
