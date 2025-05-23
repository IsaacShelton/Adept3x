use super::{Parser, annotation::Annotation, error::ParseError};
use crate::annotation::AnnotationKind;
use ast::TypeAlias;
use attributes::Privacy;
use infinite_iterator::InfinitePeekable;
use optional_string::NoneStr;
use token::{Token, TokenKind};

impl<'a, I: InfinitePeekable<Token>> Parser<'a, I> {
    pub fn parse_type_alias(
        &mut self,
        annotations: Vec<Annotation>,
    ) -> Result<TypeAlias, ParseError> {
        let source = self.input.here();
        assert!(self.input.advance().is_type_alias_keyword());

        let mut privacy = Privacy::Protected;
        let name = self.parse_identifier("for alias name after 'typealias' keyword")?;
        self.ignore_newlines();

        let params = self.parse_type_params()?;

        for annotation in annotations {
            match annotation.kind {
                AnnotationKind::Public => privacy = Privacy::Public,
                AnnotationKind::Private => privacy = Privacy::Private,
                _ => return Err(self.unexpected_annotation(&annotation, "for type alias")),
            }
        }

        self.input
            .expect(TokenKind::Assign, "after type alias name")?;

        let becomes_type = self.parse_type(NoneStr, "for type alias")?;

        Ok(TypeAlias {
            name,
            params,
            value: becomes_type,
            source,
            privacy,
        })
    }
}
