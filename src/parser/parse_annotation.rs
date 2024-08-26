use super::{
    annotation::{Annotation, AnnotationKind},
    error::{ParseError, ParseErrorKind},
    Parser,
};
use crate::{
    inflow::Inflow,
    token::{Token, TokenKind},
};

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn parse_annotation(&mut self) -> Result<Annotation, ParseError> {
        // #[annotation_name]
        // ^

        self.parse_token(TokenKind::Hash, Some("to begin annotation"))?;
        self.parse_token(TokenKind::OpenBracket, Some("to begin annotation body"))?;

        let (annotation_name, source) =
            self.parse_identifier_keep_location(Some("for annotation name"))?;

        self.parse_token(TokenKind::CloseBracket, Some("to close annotation body"))?;

        Ok(match annotation_name.as_str() {
            "foreign" => AnnotationKind::Foreign,
            "thread_local" => AnnotationKind::ThreadLocal,
            "packed" => AnnotationKind::Packed,
            "pod" => AnnotationKind::Pod,
            "abide_abi" => AnnotationKind::AbideAbi,
            _ => {
                return Err(ParseErrorKind::UnrecognizedAnnotation {
                    name: annotation_name,
                }
                .at(source))
            }
        }
        .at(source))
    }
}
