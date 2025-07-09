use super::{
    Parser,
    annotation::{Annotation, AnnotationKind},
    error::{ParseError, ParseErrorKind},
};
use ast::Given;
use infinite_iterator::InfinitePeekable;
use optional_string::NoneStr;
use source_files::Sourced;
use token::{Token, TokenKind};

impl<'a, I: InfinitePeekable<Token>> Parser<'a, I> {
    pub fn parse_annotation(&mut self) -> Result<Vec<Annotation>, ParseError> {
        // #[annotation_name]
        // ^

        self.input.expect(TokenKind::Hash, "to begin annotation")?;
        self.input
            .expect(TokenKind::OpenBracket, "to begin annotation body")?;

        let mut annotations = Vec::with_capacity(2);

        loop {
            let (annotation_name, source) = self
                .parse_identifier_keep_location("for annotation name")?
                .tuple();

            let annotation = match annotation_name.as_str() {
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

                    Given {
                        name: self
                            .input
                            .eat_polymorph()
                            .map(|name| Sourced::new(name, source)),
                        ty: self.parse_type(NoneStr, "for context")?,
                    }
                }),
                "comptime" => AnnotationKind::Comptime,
                _ => {
                    return Err(ParseErrorKind::UnrecognizedAnnotation {
                        name: annotation_name,
                    }
                    .at(source));
                }
            }
            .at(source);

            annotations.push(annotation);

            if !self.input.eat(TokenKind::Comma) {
                break;
            }
        }

        self.input
            .expect(TokenKind::CloseBracket, "to close annotation body")?;
        Ok(annotations)
    }

    pub fn parse_optional_name(&mut self) -> Option<String> {
        (self.input.peek().is_identifier() && self.input.peek_nth(1).is_identifier())
            .then(|| self.input.advance().kind.unwrap_identifier())
    }
}
