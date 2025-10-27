use super::{
    Parser,
    annotation::{Annotation, AnnotationKind},
    error::ParseError,
};
use ast::{Impl, TypeParams};
use attributes::Privacy;
use infinite_iterator::InfinitePeekable;
use optional_string::NoneStr;
use smallvec::smallvec;
use std_ext::SmallVec4;
use token::{Token, TokenKind};

impl<'a, I: InfinitePeekable<Token>> Parser<'a, I> {
    pub fn parse_impl(&mut self, annotations: SmallVec4<Annotation>) -> Result<Impl, ParseError> {
        let source = self.input.peek().source;
        self.input.advance().kind.unwrap_impl_keyword();

        let mut privacy = Privacy::Protected;

        for annotation in annotations {
            match annotation.kind {
                AnnotationKind::Public => privacy = Privacy::Public,
                AnnotationKind::Private => privacy = Privacy::Private,
                _ => {
                    return Err(self.unexpected_annotation(&annotation, "for implementation"));
                }
            }
        }

        let target = self.parse_type("trait", NoneStr)?;

        let name = self.input.eat_identifier();
        let params = TypeParams::from(self.parse_type_params()?);
        let mut body = vec![];

        self.input
            .expect(TokenKind::OpenCurly, "to begin implementation body")?;

        self.input.ignore_newlines();

        while !self.input.peek_is_or_eof(TokenKind::CloseCurly) {
            // Ignore preceeding newlines
            self.ignore_newlines();

            // Parse annotations
            let mut annotations = smallvec![];
            while self.input.peek().is_hash() {
                annotations.extend(self.parse_annotation_list()?);
                self.ignore_newlines();
            }

            body.push(self.parse_func(annotations)?);
            self.input.ignore_newlines();
        }

        self.input
            .expect(TokenKind::CloseCurly, "to end implementation body")?;

        Ok(Impl {
            name,
            params,
            target,
            source,
            privacy,
            body,
        })
    }
}
