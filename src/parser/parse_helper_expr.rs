use super::{
    annotation::{Annotation, AnnotationKind},
    error::ParseError,
    Parser,
};
use crate::{
    ast::{HelperExpr, Named, Privacy},
    inflow::Inflow,
    name::Name,
    token::{Token, TokenKind},
};

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn parse_helper_expr(
        &mut self,
        annotations: Vec<Annotation>,
    ) -> Result<Named<HelperExpr>, ParseError> {
        let source = self.source_here();
        self.input.advance();

        let mut namespace = None;
        let mut privacy = Privacy::Private;
        let name = self.parse_identifier(Some("for define name after 'define' keyword"))?;
        self.ignore_newlines();

        self.parse_token(TokenKind::Assign, Some("after name of define"))?;

        #[allow(clippy::never_loop, clippy::match_single_binding)]
        for annotation in annotations {
            match annotation.kind {
                AnnotationKind::Namespace(new_namespace) => namespace = Some(new_namespace),
                AnnotationKind::Public => privacy = Privacy::Public,
                _ => return Err(self.unexpected_annotation(&annotation, Some("for define"))),
            }
        }

        let value = self.parse_expr()?;

        Ok(Named::<HelperExpr> {
            name: Name::new(namespace, name),
            value: HelperExpr {
                value,
                source,
                is_file_local_only: false,
                privacy,
            },
        })
    }
}
