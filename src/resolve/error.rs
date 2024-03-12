use crate::line_column::Location;
use colored::Colorize;
use std::fmt::Display;

pub struct ResolveError {
    pub filename: Option<String>,
    pub location: Option<Location>,
    pub info: ErrorInfo,
}

#[derive(Clone, Debug)]
pub enum ErrorInfo {
    CannotReturnValueOfType { returning: String, expected: String },
    CannotReturnVoid { expected: String },
    UnrepresentableInteger { value: String },
    FailedToFindFunction { name: String },
    UndeclaredVariable { name: String },
    NotEnoughArgumentsToFunction { name: String },
    TooManyArgumentsToFunction { name: String },
    BadTypeForArgumentToFunction { name: String, i: usize },
    Other { message: String },
}

impl Display for ResolveError {
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

        match &self.info {
            ErrorInfo::CannotReturnValueOfType {
                returning,
                expected,
            } => {
                write!(
                    f,
                    "Cannot return value of type '{}', expected '{}'",
                    returning, expected
                )?;
            }
            ErrorInfo::CannotReturnVoid { expected } => {
                write!(f, "Cannot return 'void', expected '{}'", expected,)?;
            }
            ErrorInfo::UnrepresentableInteger { value } => {
                write!(
                    f,
                    "Failed to lower unrepresentable integer literal {}",
                    value
                )?;
            }
            ErrorInfo::FailedToFindFunction { name } => {
                write!(f, "Failed to find function '{}'", name)?;
            }
            ErrorInfo::UndeclaredVariable { name } => {
                write!(f, "Undeclared variable '{}'", name)?;
            }
            ErrorInfo::NotEnoughArgumentsToFunction { name } => {
                write!(f, "Not enough arguments for call to function '{}'", name)?;
            }
            ErrorInfo::TooManyArgumentsToFunction { name } => {
                write!(f, "Too many arguments for call to function '{}'", name)?;
            }
            ErrorInfo::BadTypeForArgumentToFunction { name, i } => {
                write!(f, "Bad type for argument #{} to function '{}'", i + 1, name)?;
            }
            ErrorInfo::Other { message } => {
                write!(f, "{}", message)?;
            }
        }

        Ok(())
    }
}
