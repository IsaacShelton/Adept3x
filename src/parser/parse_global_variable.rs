use super::{
    annotation::{Annotation, AnnotationKind},
    error::{ParseError, ParseErrorKind},
    Parser,
};
use crate::{
    ast::GlobalVar,
    inflow::Inflow,
    token::{Token, TokenKind},
};

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn parse_global_variable(
        &mut self,
        annotations: Vec<Annotation>,
    ) -> Result<GlobalVar, ParseError> {
        // my_global_name Type
        //      ^

        let mut is_foreign = false;
        let mut is_thread_local = false;

        for annotation in annotations {
            match annotation.kind {
                AnnotationKind::Foreign => is_foreign = true,
                AnnotationKind::ThreadLocal => is_thread_local = true,
                _ => {
                    return Err(self.unexpected_annotation(&annotation, Some("for global variable")))
                }
            }
        }

        let (name, source) =
            self.parse_identifier_keep_location(Some("for name of global variable"))?;

        // Better error message for trying to call functions at global scope
        if self.input.peek_is(TokenKind::OpenParen) {
            return Err(ParseErrorKind::CannotCallFunctionsAtGlobalScope.at(source));
        }

        let ast_type = self.parse_type(None::<&str>, Some("for type of global variable"))?;

        Ok(GlobalVar {
            name,
            ast_type,
            source,
            is_foreign,
            is_thread_local,
        })
    }
}