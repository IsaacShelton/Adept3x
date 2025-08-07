use crate::Parser;
use ast::FileHeader;
use compiler_version::AdeptVersion;
use diagnostics::ErrorDiagnostic;
use infinite_iterator::InfinitePeekable;
use std::str::FromStr;
use token::Token;

impl<'a, I: InfinitePeekable<Token>> Parser<'a, I> {
    pub fn parse_file_header(&mut self) -> Result<Option<FileHeader>, ErrorDiagnostic> {
        self.input.ignore_newlines();

        if !self.predict_has_file_header() {
            return Ok(None);
        }

        let mut header = FileHeader::default();
        let source = self.input.here();
        let (fields, fill_behavior) = self.parse_struct_literal_agnostic(source)?;

        if !fill_behavior.is_forbid() {
            return Err(ErrorDiagnostic::new(
                "Cannot use fill behaviors within file header",
                source,
            ));
        }

        for field in fields {
            let Some(name) = field.name else {
                return Err(ErrorDiagnostic::new(
                    "Fields are required to have names within file header",
                    source,
                ));
            };

            match name.as_str() {
                "adept" => {
                    let ast::ExprKind::String(version_string) = field.value.kind else {
                        return Err(ErrorDiagnostic::new(
                            "Adept version must be specified as a string",
                            source,
                        ));
                    };

                    header.adept = Some(AdeptVersion::from_str(&version_string).map_err(|_| {
                        ErrorDiagnostic::new(
                            "Invalid Adept version. Try \"3.0\" for example.",
                            field.value.source,
                        )
                    })?);
                }
                _ => {
                    return Err(ErrorDiagnostic::new(
                        format!("Unknown setting '{}' within file header", name),
                        source,
                    ));
                }
            }
        }

        Ok(Some(header))
    }

    fn predict_has_file_header(&mut self) -> bool {
        // TODO: Cleanup with proper newline-insensitive look-ahead

        let mut i = 0;
        if !self.input.peek_nth(i).is_open_curly() {
            return false;
        }

        i += 1;
        while self.input.peek_nth(i).is_newline() {
            i += 1;
        }

        if !self.input.peek_nth(i).is_identifier() {
            return false;
        }

        i += 1;
        while self.input.peek_nth(i).is_newline() {
            i += 1;
        }

        return self.input.peek_nth(i).is_colon();
    }
}
