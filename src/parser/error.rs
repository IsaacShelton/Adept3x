use crate::{ast::Source, show::Show, source_file_cache::SourceFileCache};
use std::fmt::Display;

#[derive(Clone, Debug)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub enum ParseErrorKind {
    UnexpectedToken {
        unexpected: String,
    },
    Expected {
        expected: String,
        for_reason: Option<String>,
        got: String,
    },
    ExpectedType {
        prefix: Option<String>,
        for_reason: Option<String>,
        got: String,
    },
    UndeclaredType {
        name: String,
    },
    UnrecognizedAnnotation {
        name: String,
    },
    ExpectedTopLevelConstruct,
    UnexpectedAnnotation {
        name: String,
        for_reason: Option<String>,
    },
    FieldSpecifiedMoreThanOnce {
        field_name: String,
    },
    ExpectedCommaInTypeParameters,
    ExpectedTypeParameters,
    Other {
        message: String,
    },
}

impl Show for ParseError {
    fn show(
        &self,
        w: &mut impl std::fmt::Write,
        source_file_cache: &SourceFileCache,
    ) -> std::fmt::Result {
        write!(
            w,
            "{}:{}:{}: error: {}",
            source_file_cache.get(self.source.key).filename(),
            self.source.location.line,
            self.source.location.column,
            self.kind
        )
    }
}

impl Display for ParseErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseErrorKind::UnexpectedToken { unexpected } => {
                write!(f, "Unexpected token {}", unexpected)?;
            }
            ParseErrorKind::Expected {
                expected,
                got,
                for_reason,
            } => {
                write!(f, "Expected {}", expected)?;

                if let Some(for_reason) = for_reason {
                    write!(f, " {}", for_reason)?;
                }

                write!(f, ", got {}", got)?;
            }
            ParseErrorKind::ExpectedType {
                prefix,
                for_reason,
                got,
            } => {
                write!(f, "Expected ")?;

                if let Some(prefix) = prefix {
                    write!(f, "{}", prefix)?;
                }

                write!(f, "type")?;

                if let Some(for_reason) = for_reason {
                    write!(f, " {}", for_reason)?;
                }

                write!(f, ", got {}", got)?;
            }
            ParseErrorKind::UndeclaredType { name } => {
                write!(f, "Undeclared type '{}'", name)?;
            }
            ParseErrorKind::UnrecognizedAnnotation { name } => {
                write!(f, "Unrecognized annotation '{}'", name)?;
            }
            ParseErrorKind::ExpectedTopLevelConstruct => {
                write!(f, "Expected top level construct")?;
            }
            ParseErrorKind::UnexpectedAnnotation { name, for_reason } => {
                write!(f, "Unexpected annotation '{}'", name)?;

                if let Some(for_reason) = for_reason {
                    write!(f, " {}", for_reason)?;
                }
            }
            ParseErrorKind::FieldSpecifiedMoreThanOnce { field_name } => {
                write!(f, "Field '{}' specified more than more", field_name)?;
            }
            ParseErrorKind::ExpectedCommaInTypeParameters => {
                write!(f, "Expected ',' during type parameters")?;
            }
            ParseErrorKind::ExpectedTypeParameters => {
                write!(f, "Expected type parameters")?;
            }
            ParseErrorKind::Other { message } => {
                write!(f, "{}", message)?;
            }
        }

        Ok(())
    }
}
