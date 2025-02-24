use super::{annotation::Annotation, error::ParseError, Parser};
use crate::{
    ast::{Privacy, TypeAlias},
    inflow::Inflow,
    parser::annotation::AnnotationKind,
    token::{Token, TokenKind},
};

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn parse_type_alias(
        &mut self,
        annotations: Vec<Annotation>,
    ) -> Result<TypeAlias, ParseError> {
        let source = self.source_here();
        assert!(self.input.advance().is_type_alias_keyword());

        let mut privacy = Privacy::Protected;
        let name = self.parse_identifier(Some("for alias name after 'typealias' keyword"))?;
        self.ignore_newlines();

        let params = self.parse_type_params()?;

        for annotation in annotations {
            match annotation.kind {
                AnnotationKind::Public => privacy = Privacy::Public,
                AnnotationKind::Private => privacy = Privacy::Private,
                _ => return Err(self.unexpected_annotation(&annotation, Some("for type alias"))),
            }
        }

        self.parse_token(TokenKind::Assign, Some("after type alias name"))?;

        let becomes_type = self.parse_type(None::<&str>, Some("for type alias"))?;

        Ok(TypeAlias {
            name,
            params,
            value: becomes_type,
            source,
            privacy,
        })
    }
}
