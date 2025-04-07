use super::{
    Parser,
    annotation::{Annotation, AnnotationKind},
    error::{ParseError, ParseErrorKind},
};
use ast::GlobalVar;
use attributes::{Privacy, SymbolOwnership};
use infinite_iterator::InfinitePeekable;
use token::{Token, TokenKind};

impl<'a, I: InfinitePeekable<Token>> Parser<'a, I> {
    pub fn parse_global_variable(
        &mut self,
        annotations: Vec<Annotation>,
    ) -> Result<GlobalVar, ParseError> {
        // my_global_name Type
        //      ^

        let mut is_foreign = false;
        let mut is_thread_local = false;
        let mut privacy = Privacy::Protected;
        let mut is_exposed = false;

        for annotation in annotations {
            match annotation.kind {
                AnnotationKind::Foreign => is_foreign = true,
                AnnotationKind::ThreadLocal => is_thread_local = true,
                AnnotationKind::Public => privacy = Privacy::Public,
                AnnotationKind::Private => privacy = Privacy::Private,
                AnnotationKind::Exposed => is_exposed = true,
                _ => {
                    return Err(
                        self.unexpected_annotation(&annotation, Some("for global variable"))
                    );
                }
            }
        }

        let (name, source) = self
            .parse_identifier_keep_location(Some("for name of global variable"))?
            .tuple();

        // Better error message for trying to call functions at global scope
        if self.input.peek_is(TokenKind::OpenParen) {
            return Err(ParseErrorKind::CannotCallFunctionsAtGlobalScope.at(source));
        }

        let ast_type = self.parse_type(None::<&str>, Some("for type of global variable"))?;

        let ownership = SymbolOwnership::from_foreign_and_exposed(is_foreign, is_exposed);

        Ok(GlobalVar {
            name,
            ast_type,
            source,
            is_thread_local,
            ownership,
            privacy,
        })
    }
}
