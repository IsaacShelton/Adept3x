use super::{annotation::Annotation, error::ParseError, Parser};
use crate::{
    ast::{HelperExpr, Named},
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

        let name = self.parse_identifier(Some("for define name after 'define' keyword"))?;
        self.ignore_newlines();

        self.parse_token(TokenKind::Assign, Some("after name of define"))?;

        #[allow(clippy::never_loop, clippy::match_single_binding)]
        for annotation in annotations {
            match annotation.kind {
                _ => return Err(self.unexpected_annotation(&annotation, Some("for define"))),
            }
        }

        let value = self.parse_expr()?;

        Ok(Named::<HelperExpr> {
            name: Name::plain(name),
            value: HelperExpr {
                value,
                source,
                is_file_local_only: false,
            },
        })
    }
}
