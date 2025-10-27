use super::{
    Parser,
    annotation::{Annotation, AnnotationKind},
    error::ParseError,
};
use ast::Namespace;
use attributes::Privacy;
use infinite_iterator::InfinitePeekable;
use std_ext::SmallVec4;
use token::{Token, TokenKind};

impl<'a, I: InfinitePeekable<Token>> Parser<'a, I> {
    pub fn parse_namespace(
        &mut self,
        annotations: SmallVec4<Annotation>,
    ) -> Result<Namespace, ParseError> {
        let source = self.input.peek().source;
        self.input.advance().kind.unwrap_namespace_keyword();

        let name = match self.input.eat_identifier() {
            Some(name) => Some(name),
            None => {
                if self.input.eat(TokenKind::Multiply) {
                    None
                } else {
                    return Err(ParseError::other(
                        "Expected name of namepace or '*' after 'namespace' keyword",
                        source,
                    ));
                }
            }
        };

        let mut privacy = None;

        for annotation in annotations {
            match annotation.kind {
                AnnotationKind::Public => privacy = Some(Privacy::Public),
                AnnotationKind::Protected => privacy = Some(Privacy::Protected),
                AnnotationKind::Private => privacy = Some(Privacy::Private),
                _ => {
                    return Err(self.unexpected_annotation(&annotation, "for namespace"));
                }
            }
        }

        let items = if self.input.eat(TokenKind::Assign) {
            self.parse_expr()?.into()
        } else {
            self.input
                .expect(TokenKind::OpenCurly, "to begin namespace")?;

            let items = self.parse_namespace_items()?;

            self.ignore_newlines();

            self.input
                .expect(TokenKind::CloseCurly, "to end namespace")?;
            items.into()
        };

        Ok(Namespace {
            name,
            items,
            source,
            privacy,
        })
    }
}
