use super::{
    annotation::{Annotation, AnnotationKind},
    error::{ParseError, ParseErrorKind},
    Parser,
};
use crate::{
    ast::{Field, Privacy, Structure, TypeKind, TypeParameter},
    inflow::Inflow,
    token::{Token, TokenKind},
};
use indexmap::IndexMap;

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn parse_type_parameter(
        &mut self,
        generics: &mut IndexMap<String, TypeParameter>,
    ) -> Result<(), ParseError> {
        if !self.input.peek().is_polymorph() {
            return Err(ParseErrorKind::Expected {
                expected: "polymorph".into(),
                for_reason: Some("for generic type parameter".into()),
                got: self.input.peek().to_string(),
            }
            .at(self.input.peek().source));
        }

        let token = self.input.advance();
        let polymorph = token.kind.unwrap_polymorph();
        let mut constraints = vec![];

        if self.input.eat(TokenKind::Colon) {
            loop {
                constraints.push(self.parse_type(None::<&str>, Some("for polymorph constraint"))?);

                if let TypeKind::Polymorph(..) = constraints.last().unwrap().kind {
                    return Err(ParseErrorKind::PolymorphsCannotBeUsedAsConstraints
                        .at(constraints.last().unwrap().source));
                }

                if !self.input.eat(TokenKind::Add) {
                    break;
                }
            }
        }

        // TODO: CLEANUP: Clean up this part to not clone unless necessary
        if generics
            .insert(polymorph.clone(), TypeParameter::new(constraints))
            .is_some()
        {
            return Err(
                ParseErrorKind::GenericTypeParameterAlreadyExists { name: polymorph }
                    .at(token.source),
            );
        }

        Ok(())
    }

    pub fn parse_structure(
        &mut self,
        annotations: Vec<Annotation>,
    ) -> Result<Structure, ParseError> {
        let source = self.source_here();
        self.input.advance();

        let name = self.parse_identifier(Some("for struct name after 'struct' keyword"))?;
        self.ignore_newlines();

        let mut is_packed = false;
        let mut privacy = Privacy::Private;

        for annotation in annotations {
            match annotation.kind {
                AnnotationKind::Packed => is_packed = true,
                AnnotationKind::Public => privacy = Privacy::Public,
                _ => return Err(self.unexpected_annotation(&annotation, Some("for struct"))),
            }
        }

        let mut fields = IndexMap::new();

        self.ignore_newlines();

        let mut parameters = IndexMap::new();

        if self.input.eat(TokenKind::OpenAngle) {
            self.ignore_newlines();

            loop {
                if self.input.peek_is_or_eof(TokenKind::GreaterThan) {
                    break;
                }

                self.parse_type_parameter(&mut parameters)?;

                if !self.input.eat(TokenKind::Comma) {
                    continue;
                }

                self.ignore_newlines();
                continue;
            }

            if !self.input.eat(TokenKind::GreaterThan) {
                return Err(ParseErrorKind::Expected {
                    expected: ">".into(),
                    for_reason: Some(" to close generics list".into()),
                    got: self.input.peek().to_string(),
                }
                .at(self.input.peek().source));
            }
        }

        self.parse_token(TokenKind::OpenParen, Some("to begin struct fields"))?;

        while !self.input.peek_is_or_eof(TokenKind::CloseParen) {
            if !fields.is_empty() {
                self.parse_token(TokenKind::Comma, Some("to separate struct fields"))?;
                self.ignore_newlines();
            }

            let source = self.source_here();
            let field_name = self.parse_identifier(Some("for field name"))?;

            self.ignore_newlines();
            let field_type = self.parse_type(None::<&str>, Some("for field type"))?;
            self.ignore_newlines();

            fields.insert(
                field_name,
                Field {
                    ast_type: field_type,
                    privacy: Default::default(),
                    source,
                },
            );
        }

        self.parse_token(TokenKind::CloseParen, Some("to end struct fields"))?;

        Ok(Structure {
            name,
            fields,
            is_packed,
            parameters,
            source,
            privacy,
        })
    }
}
