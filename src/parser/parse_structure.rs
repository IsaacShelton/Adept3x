use super::{
    annotation::{Annotation, AnnotationKind},
    error::ParseError,
    Parser,
};
use crate::{
    ast::{Field, Privacy, Struct},
    inflow::Inflow,
    token::{Token, TokenKind},
};
use indexmap::IndexMap;

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn parse_structure(&mut self, annotations: Vec<Annotation>) -> Result<Struct, ParseError> {
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

        self.ignore_newlines();

        let parameters = self.parse_type_parameters()?;
        self.parse_token(TokenKind::OpenParen, Some("to begin struct fields"))?;
        self.ignore_newlines();

        let mut fields = IndexMap::new();

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

        Ok(Struct {
            name,
            fields,
            is_packed,
            params: parameters,
            source,
            privacy,
        })
    }
}
