use super::{
    Parser,
    annotation::{Annotation, AnnotationKind},
    error::ParseError,
};
use ast::{Field, Struct};
use attributes::Privacy;
use indexmap::IndexMap;
use infinite_iterator::InfinitePeekable;
use optional_string::NoneStr;
use token::{Token, TokenKind};

impl<'a, I: InfinitePeekable<Token>> Parser<'a, I> {
    pub fn parse_structure(&mut self, annotations: Vec<Annotation>) -> Result<Struct, ParseError> {
        let source = self.input.here();
        self.input.advance();

        let name = self.parse_identifier("for struct name after 'struct' keyword")?;
        self.ignore_newlines();

        let mut is_packed = false;
        let mut privacy = Privacy::Protected;

        for annotation in annotations {
            match annotation.kind {
                AnnotationKind::Packed => is_packed = true,
                AnnotationKind::Public => privacy = Privacy::Public,
                AnnotationKind::Private => privacy = Privacy::Private,
                _ => return Err(self.unexpected_annotation(&annotation, "for struct")),
            }
        }

        self.ignore_newlines();

        let params = self.parse_type_params()?;
        self.input
            .expect(TokenKind::OpenParen, "to begin struct fields")?;
        self.ignore_newlines();

        let mut fields = IndexMap::new();

        while !self.input.peek_is_or_eof(TokenKind::CloseParen) {
            if !fields.is_empty() {
                self.input
                    .expect(TokenKind::Comma, "to separate struct fields")?;

                self.ignore_newlines();

                if self.input.peek_is_or_eof(TokenKind::CloseParen) {
                    break;
                }
            }

            let source = self.input.here();
            let field_name = self.parse_identifier("for field name")?;

            self.ignore_newlines();
            let field_type = self.parse_type(NoneStr, "for field type")?;
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

        self.input
            .expect(TokenKind::CloseParen, "to end struct fields")?;

        Ok(Struct {
            name,
            fields,
            is_packed,
            params: params.into(),
            source,
            privacy,
        })
    }
}
