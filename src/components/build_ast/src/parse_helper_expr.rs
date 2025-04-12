use super::{
    Parser,
    annotation::{Annotation, AnnotationKind},
    error::ParseError,
};
use ast::ExprAlias;
use attributes::Privacy;
use infinite_iterator::InfinitePeekable;
use token::{Token, TokenKind};

impl<'a, I: InfinitePeekable<Token>> Parser<'a, I> {
    pub fn parse_helper_expr(
        &mut self,
        annotations: Vec<Annotation>,
    ) -> Result<ExprAlias, ParseError> {
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

        Ok(ExprAlias {
            name,
            value,
            source,
            is_file_local_only: false,
            privacy,
        })
    }
}
