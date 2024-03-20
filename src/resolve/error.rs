use crate::line_column::Location;
use colored::Colorize;
use std::fmt::Display;

pub struct ResolveError {
    pub filename: Option<String>,
    pub location: Option<Location>,
    pub kind: ResolveErrorKind,
}

#[derive(Clone, Debug)]
pub enum ResolveErrorKind {
    CannotReturnValueOfType { returning: String, expected: String },
    CannotReturnVoid { expected: String },
    UnrepresentableInteger { value: String },
    FailedToFindFunction { name: String },
    UndeclaredVariable { name: String },
    UndeclaredType { name: String },
    NotEnoughArgumentsToFunction { name: String },
    TooManyArgumentsToFunction { name: String },
    BadTypeForArgumentToFunction { name: String, i: usize },
    BinaryOperatorMismatch { left: String, right: String },
    CannotBinaryOperator { left: String, right: String },
    TypeMismatch { left: String, right: String },
    CannotAssignValueOfType { from: String, to: String },
    CannotMutate,
    CannotGetFieldOfNonPlainOldDataType { bad_type: String },
    FieldIsPrivate { field_name: String },
    FieldDoesNotExist { field_name: String },
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

        match &self.kind {
            ResolveErrorKind::CannotReturnValueOfType {
                returning,
                expected,
            } => {
                write!(
                    f,
                    "Cannot return value of type '{}', expected '{}'",
                    returning, expected
                )?;
            }
            ResolveErrorKind::CannotReturnVoid { expected } => {
                write!(f, "Cannot return 'void', expected '{}'", expected,)?;
            }
            ResolveErrorKind::UnrepresentableInteger { value } => {
                write!(
                    f,
                    "Failed to lower unrepresentable integer literal {}",
                    value
                )?;
            }
            ResolveErrorKind::FailedToFindFunction { name } => {
                write!(f, "Failed to find function '{}'", name)?;
            }
            ResolveErrorKind::UndeclaredVariable { name } => {
                write!(f, "Undeclared variable '{}'", name)?;
            }
            ResolveErrorKind::UndeclaredType { name } => {
                write!(f, "Undeclared type '{}'", name)?;
            }
            ResolveErrorKind::NotEnoughArgumentsToFunction { name } => {
                write!(f, "Not enough arguments for call to function '{}'", name)?;
            }
            ResolveErrorKind::TooManyArgumentsToFunction { name } => {
                write!(f, "Too many arguments for call to function '{}'", name)?;
            }
            ResolveErrorKind::BadTypeForArgumentToFunction { name, i } => {
                write!(f, "Bad type for argument #{} to function '{}'", i + 1, name)?;
            }
            ResolveErrorKind::BinaryOperatorMismatch { left, right } => {
                write!(
                    f,
                    "Mismatching types '{}' and '{}' for binary operator",
                    left, right
                )?;
            }
            ResolveErrorKind::CannotBinaryOperator { left, right } => {
                write!(
                    f,
                    "Cannot perform binary operator on types '{}' and '{}'",
                    left, right
                )?;
            }
            ResolveErrorKind::TypeMismatch { left, right } => {
                write!(f, "Mismatching types '{}' and '{}'", left, right)?;
            }
            ResolveErrorKind::CannotAssignValueOfType { from, to } => {
                write!(f, "Cannot assign value of type '{}' to '{}'", from, to)?;
            }
            ResolveErrorKind::CannotMutate => {
                write!(f, "Cannot mutate value")?;
            }
            ResolveErrorKind::CannotGetFieldOfNonPlainOldDataType { bad_type } => {
                write!(
                    f,
                    "Cannot get field of non-plain-old-data type '{}'",
                    bad_type
                )?;
            }
            ResolveErrorKind::FieldIsPrivate { field_name } => {
                write!(f, "Field '{}' is private", field_name)?;
            }
            ResolveErrorKind::FieldDoesNotExist { field_name } => {
                write!(f, "Field '{}' does not exist", field_name)?;
            }
            ResolveErrorKind::Other { message } => {
                write!(f, "{}", message)?;
            }
        }

        Ok(())
    }
}
