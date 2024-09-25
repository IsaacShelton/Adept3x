use super::{annotation::Annotation, error::ParseError, Parser};
use crate::{
    ast::{Named, TypeAlias},
    inflow::Inflow,
    name::Name,
    parser::annotation::AnnotationKind,
    token::{Token, TokenKind},
};

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn parse_type_alias(
        &mut self,
        annotations: Vec<Annotation>,
    ) -> Result<Named<TypeAlias>, ParseError> {
        let source = self.source_here();
        assert!(self.input.advance().is_type_alias_keyword());

        let mut namespace = None;
        let name = self.parse_identifier(Some("for alias name after 'typealias' keyword"))?;
        self.ignore_newlines();

        #[allow(clippy::never_loop, clippy::match_single_binding)]
        for annotation in annotations {
            match annotation.kind {
                AnnotationKind::Namespace(new_namespace) => namespace = Some(new_namespace),
                _ => return Err(self.unexpected_annotation(&annotation, Some("for type alias"))),
            }
        }

        self.parse_token(TokenKind::Assign, Some("after type alias name"))?;

        let becomes_type = self.parse_type(None::<&str>, Some("for type alias"))?;

        Ok(Named::<TypeAlias> {
            name: Name::new(namespace, name),
            value: TypeAlias {
                value: becomes_type,
                source,
            },
        })
    }
}
