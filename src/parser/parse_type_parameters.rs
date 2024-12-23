use super::{
    error::{ParseError, ParseErrorKind},
    Parser,
};
use crate::{
    ast::{TypeKind, TypeParameter},
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

    pub fn parse_type_parameters(&mut self) -> Result<IndexMap<String, TypeParameter>, ParseError> {
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

        Ok(parameters)
    }
}
