use super::{Parser, annotation::Annotation, error::ParseError};
use crate::annotation::AnnotationKind;
use ast::{Enum, EnumMember};
use attributes::Privacy;
use indexmap::IndexMap;
use infinite_iterator::InfinitePeekable;
use num::{BigInt, Zero};
use std_ext::SmallVec4;
use token::{Token, TokenKind};

impl<'a, I: InfinitePeekable<Token>> Parser<'a, I> {
    pub fn parse_enum(&mut self, annotations: SmallVec4<Annotation>) -> Result<Enum, ParseError> {
        let source = self.input.here();
        assert!(self.input.advance().is_enum_keyword());

        let mut privacy = Privacy::Protected;
        let name = self.parse_identifier("for enum name after 'enum' keyword")?;
        self.ignore_newlines();

        for annotation in annotations {
            match annotation.kind {
                AnnotationKind::Public => privacy = Privacy::Public,
                AnnotationKind::Private => privacy = Privacy::Private,
                _ => return Err(self.unexpected_annotation(&annotation, "for enum")),
            }
        }

        let mut members = IndexMap::new();
        let mut next_value = BigInt::zero();

        self.input.expect(TokenKind::OpenParen, "after enum name")?;

        while !self.input.peek_is_or_eof(TokenKind::CloseParen) {
            let member_name = self.parse_identifier("for enum member")?;

            let value = next_value.clone();
            next_value += 1;

            members.insert(
                member_name,
                EnumMember {
                    value,
                    explicit_value: false,
                },
            );

            if !self.input.peek_is(TokenKind::CloseParen) {
                self.input.expect(TokenKind::Comma, "after enum member")?;
            }
        }

        self.input
            .expect(TokenKind::CloseParen, "to close enum body")?;

        Ok(Enum {
            name,
            backing_type: None,
            members,
            source,
            privacy,
        })
    }
}
