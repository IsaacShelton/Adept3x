use crate::line_column::Location;
use colored::Colorize;
use std::fmt::Display;

#[derive(Clone, Debug)]
pub struct ParseError {
    pub filename: Option<String>,
    pub location: Option<Location>,
    pub info: ErrorInfo,
}

#[derive(Clone, Debug)]
pub enum ErrorInfo {
    UnexpectedToken {
        unexpected: Option<String>,
    },
    Expected {
        expected: String,
        for_reason: Option<String>,
        got: Option<String>,
    },
    ExpectedType {
        prefix: Option<String>,
        for_reason: Option<String>,
        got: Option<String>,
    },
    UndeclaredType {
        name: String,
    },
    UnrecognizedAnnotation {
        name: String,
    },
    Other {
        message: String,
    },
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ErrorInfo::*;

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

        match &self.info {
            UnexpectedToken { unexpected } => {
                write!(f, "Unexpected token")?;

                if let Some(token) = unexpected {
                    write!(f, " {}", token)?;
                } else {
                    write!(f, " end-of-file")?;
                }
            }
            Expected {
                expected,
                got,
                for_reason,
            } => {
                write!(f, "Expected {}", expected)?;

                if let Some(for_reason) = for_reason {
                    write!(f, " {}", for_reason)?;
                }

                if let Some(got) = got {
                    write!(f, ", got {}", got)?;
                } else {
                    write!(f, ", got end-of-file")?;
                }
            }
            ExpectedType { prefix, for_reason, got } => {
                write!(f, "Expected ")?;

                if let Some(prefix) = prefix {
                    write!(f, "{}", prefix)?;
                }

                write!(f, "type")?;

                if let Some(for_reason) = for_reason {
                    write!(f, " {}", for_reason)?;
                }

                if let Some(got) = got {
                    write!(f, ", got {}", got)?;
                } else {
                    write!(f, ", got end-of-file")?;
                }
            }
            UndeclaredType { name } => {
                write!(f, "Undeclared type '{}'", name)?;
            }
            UnrecognizedAnnotation { name } => {
                write!(f, "Unrecognized annotation '{}'", name)?;
            }
            Other { message } => {
                write!(f, "{}", message)?;
            }
        }

        Ok(())
    }
}
