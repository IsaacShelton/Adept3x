use super::{
    Parser,
    annotation::{Annotation, AnnotationKind},
    error::{ParseError, ParseErrorKind},
};
use ast::Given;
use infinite_iterator::InfinitePeekable;
use optional_string::NoneStr;
use smallvec::SmallVec;
use source_files::Sourced;
use std_ext::SmallVec4;
use token::{Token, TokenKind};

impl<'a, I: InfinitePeekable<Token>> Parser<'a, I> {
    pub fn parse_annotation_list(&mut self) -> Result<SmallVec4<Annotation>, ParseError> {
        // #[annotation_name, annotation_name_2, annotation_name_3]
        // ^

        self.input.expect(TokenKind::Hash, "to begin annotation")?;
        self.input
            .expect(TokenKind::OpenBracket, "to begin annotation body")?;

        let mut annotations = SmallVec::with_capacity(2);

        loop {
            annotations.push(self.parse_annotation_in_list()?);

            if !self.input.eat(TokenKind::Comma) {
                break;
            }
        }

        self.input
            .expect(TokenKind::CloseBracket, "to close annotation body")?;
        Ok(annotations)
    }

    pub fn parse_annotation_in_list(&mut self) -> Result<Annotation, ParseError> {
        let (annotation_name, source) = self
            .parse_identifier_keep_location("for annotation name")?
            .tuple();

        Ok(match annotation_name.as_str() {
            "foreign" => AnnotationKind::Foreign,
            "exposed" => AnnotationKind::Exposed,
            "thread_local" => AnnotationKind::ThreadLocal,
            "packed" => AnnotationKind::Packed,
            "abide_abi" => AnnotationKind::AbideAbi,
            "public" => AnnotationKind::Public,
            "private" => AnnotationKind::Private,
            "template" => AnnotationKind::Template,
            "using" => AnnotationKind::Using({
                let source = self.input.here();

                Box::new(Given {
                    name: self
                        .input
                        .eat_polymorph()
                        .map(|name| Sourced::new(name, source)),
                    ty: self.parse_type(NoneStr, "for context")?,
                })
            }),
            "comptime" => AnnotationKind::Comptime,
            _ => {
                return Err(ParseErrorKind::UnrecognizedAnnotation {
                    name: annotation_name,
                }
                .at(source));
            }
        }
        .at(source))
    }

    pub fn parse_optional_name(&mut self) -> Option<String> {
        (self.input.peek().is_identifier() && self.input.peek_nth(1).is_identifier())
            .then(|| self.input.advance().kind.unwrap_identifier())
    }
}
