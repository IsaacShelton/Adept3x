use super::{
    Parser,
    annotation::{Annotation, AnnotationKind},
    error::ParseError,
};
use ast::Namespace;
use attributes::Privacy;
use infinite_iterator::InfinitePeekable;
use token::{Token, TokenKind};

impl<'a, I: InfinitePeekable<Token>> Parser<'a, I> {
    pub fn parse_namespace(
        &mut self,
        annotations: Vec<Annotation>,
    ) -> Result<Namespace, ParseError> {
        let source = self.input.peek().source;
        self.input.advance().kind.unwrap_namespace_keyword();

        let Some(name) = self.input.eat_identifier() else {
            return Err(ParseError::other(
                "Expected name of namepace after 'namespace' keyword",
                source,
            ));
        };

        let mut privacy = Privacy::Protected;

        for annotation in annotations {
            match annotation.kind {
                AnnotationKind::Public => privacy = Privacy::Public,
                AnnotationKind::Private => privacy = Privacy::Private,
                _ => {
                    return Err(self.unexpected_annotation(&annotation, "for namespace"));
                }
            }
        }

        self.input
            .expect(TokenKind::OpenCurly, "to begin namespace")?;

        let items = self.parse_namespace_items()?;

        self.input
            .expect(TokenKind::CloseCurly, "to end namespace")?;

        Ok(Namespace {
            name,
            items,
            source,
            privacy,
        })
    }
}
