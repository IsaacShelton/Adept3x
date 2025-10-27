use super::{Parser, annotation::Annotation, error::ParseError};
use ast::{Linkset, LinksetEntry};
use infinite_iterator::InfinitePeekable;
use std::path::Path;
use std_ext::SmallVec4;
use token::{Token, TokenKind};

impl<'a, I: InfinitePeekable<Token>> Parser<'a, I> {
    pub fn parse_linkset(
        &mut self,
        annotations: SmallVec4<Annotation>,
    ) -> Result<Linkset, ParseError> {
        self.input.advance().kind.unwrap_linkset_keyword();

        for annotation in annotations {
            match annotation.kind {
                _ => {
                    return Err(self.unexpected_annotation(&annotation, "for namespace"));
                }
            }
        }

        self.input
            .expect(TokenKind::OpenCurly, "to begin linkset")?;

        let mut entries = vec![];

        while !self.input.peek_is_or_eof(TokenKind::CloseCurly) {
            self.input.ignore_newlines();

            let key = self.input.eat_identifier();

            match key.as_ref().map(|entry_type| entry_type.as_str()) {
                Some("file") => {
                    let token = self.input.advance();

                    let TokenKind::String(literal) = &token.kind else {
                        return Err(ParseError::expected("filename", None::<&str>, token));
                    };

                    let relative_filename = &literal.value;

                    let filename = Path::new(self.input.filename())
                        .parent()
                        .unwrap()
                        .join(relative_filename);

                    entries.push(LinksetEntry::File(filename))
                }
                Some("library") => {
                    let token = self.input.advance();

                    let TokenKind::String(literal) = &token.kind else {
                        return Err(ParseError::expected("library", None::<&str>, token));
                    };

                    entries.push(LinksetEntry::Library(literal.value.clone()))
                }
                Some("framework") => {
                    let token = self.input.advance();

                    let TokenKind::String(literal) = &token.kind else {
                        return Err(ParseError::expected("framework", None::<&str>, token));
                    };

                    entries.push(LinksetEntry::Framework(literal.value.clone()))
                }
                Some(_) => {
                    return Err(ParseError::expected(
                        "valid linkset entry type",
                        None::<&str>,
                        self.input.peek(),
                    ));
                }
                None => {
                    return Err(ParseError::expected(
                        "identifier",
                        Some("for linkset entry type"),
                        self.input.peek(),
                    ));
                }
            }

            self.input.ignore_newlines();
        }

        self.input
            .expect(TokenKind::CloseCurly, "to close linkset")?;

        Ok(Linkset { entries })
    }
}
