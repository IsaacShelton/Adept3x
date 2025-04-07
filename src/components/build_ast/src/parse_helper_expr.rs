use super::{
    Parser,
    annotation::{Annotation, AnnotationKind},
    error::ParseError,
};
use ast::HelperExpr;
use attributes::Privacy;
use inflow::Inflow;
use token::{Token, TokenKind};

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn parse_helper_expr(
        &mut self,
        annotations: Vec<Annotation>,
    ) -> Result<HelperExpr, ParseError> {
        let source = self.source_here();
        self.input.advance();

        let mut privacy = Privacy::Protected;
        let name = self.parse_identifier(Some("for define name after 'define' keyword"))?;
        self.ignore_newlines();

        self.parse_token(TokenKind::Assign, Some("after name of define"))?;

        #[allow(clippy::never_loop, clippy::match_single_binding)]
        for annotation in annotations {
            match annotation.kind {
                AnnotationKind::Public => privacy = Privacy::Public,
                AnnotationKind::Private => privacy = Privacy::Private,
                _ => return Err(self.unexpected_annotation(&annotation, Some("for define"))),
            }
        }

        let value = self.parse_expr()?;

        Ok(HelperExpr {
            name,
            value,
            source,
            is_file_local_only: false,
            privacy,
        })
    }
}
