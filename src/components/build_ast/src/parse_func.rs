use super::{
    Parser,
    annotation::{Annotation, AnnotationKind},
    error::ParseError,
};
use ast::{Func, FuncHead, TypeKind};
use attributes::{Privacy, SymbolOwnership, Tag};
use infinite_iterator::InfinitePeekable;
use token::{Token, TokenKind};

impl<'a, I: InfinitePeekable<Token>> Parser<'a, I> {
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
        let mut is_exposed = false;
        let mut abide_abi = false;
        let mut privacy = Privacy::Protected;
        let mut givens = vec![];

        for annotation in annotations {
            match annotation.kind {
                AnnotationKind::Foreign => is_foreign = true,
                AnnotationKind::Exposed => is_exposed = true,
                AnnotationKind::AbideAbi => abide_abi = true,
                AnnotationKind::Public => privacy = Privacy::Public,
                AnnotationKind::Private => privacy = Privacy::Private,
                AnnotationKind::Using(given) => {
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

        let type_params = self.parse_type_params()?;
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
            self.parse_type(Some("return"), Some("for function"))?
        };

        let stmts = (!is_foreign)
            .then(|| self.parse_block("function"))
            .transpose()?
            .unwrap_or_default();

        let tag = (name == "main").then_some(Tag::Main);
        let ownership = SymbolOwnership::from_foreign_and_exposed(is_foreign, is_exposed);

        Ok(Func {
            head: FuncHead {
                name,
                type_params,
                givens,
                params,
                return_type,
                ownership,
                source,
                abide_abi,
                tag,
                privacy,
            },
            stmts,
        })
    }
}
