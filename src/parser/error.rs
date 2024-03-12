use crate::line_column::Location;
use colored::Colorize;
use std::fmt::Display;

#[derive(Clone, Debug)]
pub struct ParseError {
    pub filename: Option<String>,
    pub location: Option<Location>,
    pub kind: ParseErrorKind,
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
    Other {
        message: String,
    },
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(filename) = &self.filename {
            write!(f, "{}:", filename)?;
        }

        if let Some(location) = self.location {
            write!(f, "{}:{}:", location.line, location.column)?;
        }

        if self.filename.is_some() || self.location.is_some() {
            write!(f, " ")?;
        }

        write!(f, "{}", "error: ".bright_red())?;

        match &self.kind {
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
            ParseErrorKind::Other { message } => {
                write!(f, "{}", message)?;
            }
        }

        Ok(())
    }
}
