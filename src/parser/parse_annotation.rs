use super::{
    annotation::{Annotation, AnnotationKind},
    error::{ParseError, ParseErrorKind},
    Parser,
};
use crate::{
    ast::Given,
    inflow::Inflow,
    token::{Token, TokenKind},
};

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn parse_annotation(&mut self) -> Result<Vec<Annotation>, ParseError> {
        // #[annotation_name]
        // ^

        self.parse_token(TokenKind::Hash, Some("to begin annotation"))?;
        self.parse_token(TokenKind::OpenBracket, Some("to begin annotation body"))?;
        let mut annotations = Vec::with_capacity(2);

        loop {
            let (annotation_name, source) = self
                .parse_identifier_keep_location(Some("for annotation name"))?
                .tuple();

            let annotation = match annotation_name.as_str() {
                "foreign" => AnnotationKind::Foreign,
                "thread_local" => AnnotationKind::ThreadLocal,
                "packed" => AnnotationKind::Packed,
                "abide_abi" => AnnotationKind::AbideAbi,
                "public" => AnnotationKind::Public,
                "private" => AnnotationKind::Private,
                "template" => AnnotationKind::Template,
                "using" => AnnotationKind::Using({
                    let source = self.input.peek().source;

                    Given {
                        name: self.input.eat_polymorph().map(|name| (name, source)),
                        ty: self.parse_type(None::<&str>, Some("for context"))?,
                    }
                }),
                _ => {
                    return Err(ParseErrorKind::UnrecognizedAnnotation {
                        name: annotation_name,
                    }
                    .at(source))
                }
            }
            .at(source);

            annotations.push(annotation);

            if !self.input.eat(TokenKind::Comma) {
                break;
            }
        }

        self.parse_token(TokenKind::CloseBracket, Some("to close annotation body"))?;
        Ok(annotations)
    }

    pub fn parse_optional_name(&mut self) -> Option<String> {
        (self.input.peek().is_identifier() && self.input.peek_nth(1).is_identifier())
            .then(|| self.input.advance().kind.unwrap_identifier())
    }
}
