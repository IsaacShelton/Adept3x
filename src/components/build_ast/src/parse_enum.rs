use super::{Parser, annotation::Annotation, error::ParseError};
use crate::annotation::AnnotationKind;
use ast::{Enum, EnumMember};
use attributes::Privacy;
use indexmap::IndexMap;
use inflow::Inflow;
use num::{BigInt, Zero};
use token::{Token, TokenKind};

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn parse_enum(&mut self, annotations: Vec<Annotation>) -> Result<Enum, ParseError> {
        let source = self.source_here();
        assert!(self.input.advance().is_enum_keyword());

        let mut privacy = Privacy::Protected;
        let name = self.parse_identifier(Some("for enum name after 'enum' keyword"))?;
        self.ignore_newlines();

        for annotation in annotations {
            match annotation.kind {
                AnnotationKind::Public => privacy = Privacy::Public,
                AnnotationKind::Private => privacy = Privacy::Private,
                _ => return Err(self.unexpected_annotation(&annotation, Some("for enum"))),
            }
        }

        let mut members = IndexMap::new();

        self.parse_token(TokenKind::OpenParen, Some("after enum name"))?;
        let mut next_value = BigInt::zero();

        while !self.input.peek_is_or_eof(TokenKind::CloseParen) {
            let member_name = self.parse_identifier(Some("for enum member"))?;

            let value = next_value.clone();
            next_value += 1;

            members.insert(
                member_name,
                EnumMember {
                    value,
                    explicit_value: false,
                },
            );

            if !self.input.eat(TokenKind::Comma) && !self.input.peek_is(TokenKind::CloseParen) {
                let got = self.input.advance();
                return Err(ParseError::expected(
                    TokenKind::Comma,
                    Some("after enum member"),
                    got,
                ));
            }
        }

        self.parse_token(TokenKind::CloseParen, Some("to close enum body"))?;

        Ok(Enum {
            name,
            backing_type: None,
            members,
            source,
            privacy,
        })
    }
}
