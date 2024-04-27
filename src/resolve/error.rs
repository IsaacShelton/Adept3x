use crate::{ast::Source, line_column::Location, source_file_cache::SourceFileCache};
use colored::Colorize;
use itertools::Itertools;
use std::fmt::Display;

pub struct ResolveError {
    pub filename: Option<String>,
    pub location: Option<Location>,
    pub kind: ResolveErrorKind,
}

impl ResolveError {
    pub fn new(
        source_file_cache: &SourceFileCache,
        source: Source,
        kind: ResolveErrorKind,
    ) -> Self {
        Self {
            filename: Some(source_file_cache.get(source.key).filename().to_string()),
            location: Some(source.location),
            kind,
        }
    }
}

#[derive(Clone, Debug)]
pub enum ResolveErrorKind {
    CannotReturnValueOfType {
        returning: String,
        expected: String,
    },
    CannotReturnVoid {
        expected: String,
    },
    UnrepresentableInteger {
        value: String,
    },
    FailedToFindFunction {
        name: String,
    },
    UndeclaredVariable {
        name: String,
    },
    UndeclaredType {
        name: String,
    },
    NotEnoughArgumentsToFunction {
        name: String,
    },
    TooManyArgumentsToFunction {
        name: String,
    },
    BadTypeForArgumentToFunction {
        name: String,
        i: usize,
        expected: String,
        got: String,
    },
    IncompatibleTypesForBinaryOperator {
        operator: String,
        left: String,
        right: String,
    },
    CannotBinaryOperator {
        left: String,
        right: String,
    },
    TypeMismatch {
        left: String,
        right: String,
    },
    CannotAssignValueOfType {
        from: String,
        to: String,
    },
    CannotMutate {
        bad_type: String,
    },
    CannotGetFieldOfType {
        bad_type: String,
    },
    CannotCreatePlainOldDataOfNonStructure {
        bad_type: String,
    },
    FieldIsPrivate {
        field_name: String,
    },
    FieldDoesNotExist {
        field_name: String,
    },
    CannotCreateStructLiteralForNonPlainOldDataStructure {
        bad_type: String,
    },
    MissingFields {
        fields: Vec<String>,
    },
    CannotUseUninitializedValue,
    CannotUseUninitializedVariable {
        variable_name: String,
    },
    CannotPerformUnaryOperationForType {
        operator: String,
        bad_type: String,
    },
    CannotPerformBinaryOperationForType {
        operator: String,
        bad_type: String,
    },
    MismatchingYieldedTypes {
        got: Vec<String>,
    },
    StringTypeNotDefined,
    ExpectedTypeForField {
        structure: String,
        field_name: String,
        expected: String,
    },
    CannotAccessMemberOf {
        bad_type: String,
    },
    ExpectedIndexOfType {
        expected: String,
        got: String,
    },
    ExpectedTypeForSide {
        side: String,
        operator: String,
        expected: String,
        got: String,
    },
    Other {
        message: String,
    },
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
                write!(f, "Cannot return 'void', expected '{}'", expected)?;
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
            ResolveErrorKind::BadTypeForArgumentToFunction {
                name,
                i,
                expected,
                got,
            } => {
                write!(
                    f,
                    "Bad type for argument #{} to function '{}' (expected '{}' but got '{}')",
                    i + 1,
                    name,
                    expected,
                    got
                )?;
            }
            ResolveErrorKind::IncompatibleTypesForBinaryOperator {
                operator,
                left,
                right,
            } => {
                write!(
                    f,
                    "Incompatible types '{}' and '{}' for '{}'",
                    left, right, operator
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
            ResolveErrorKind::CannotMutate { bad_type } => {
                write!(f, "Cannot mutate value of type '{}'", bad_type)?;
            }
            ResolveErrorKind::CannotGetFieldOfType { bad_type } => {
                write!(f, "Cannot get field of type '{}'", bad_type)?;
            }
            ResolveErrorKind::CannotCreatePlainOldDataOfNonStructure { bad_type } => {
                write!(
                    f,
                    "Cannot create plain-old-data type of non-struct type '{}'",
                    bad_type
                )?;
            }
            ResolveErrorKind::FieldIsPrivate { field_name } => {
                write!(f, "Field '{}' is private", field_name)?;
            }
            ResolveErrorKind::FieldDoesNotExist { field_name } => {
                write!(f, "Field '{}' does not exist", field_name)?;
            }
            ResolveErrorKind::CannotCreateStructLiteralForNonPlainOldDataStructure { bad_type } => {
                write!(
                    f,
                    "Cannot create struct literal for non-plain-old-data structure '{}'",
                    bad_type
                )?;
            }
            ResolveErrorKind::MissingFields { fields } => {
                let first_missing_field_names = fields
                    .iter()
                    .take(5)
                    .map(|field_name| format!("'{}'", field_name))
                    .join(", ");

                match fields.len() {
                    0..=4 => write!(f, "Missing fields - {}", first_missing_field_names)?,
                    _ => write!(f, "Missing fields - {}, ...", first_missing_field_names)?,
                }
            }
            ResolveErrorKind::CannotUseUninitializedValue => {
                write!(f, "Cannot use uninitialized value")?;
            }
            ResolveErrorKind::CannotUseUninitializedVariable { variable_name } => {
                write!(f, "Cannot use uninitialized variable '{}'", variable_name)?;
            }
            ResolveErrorKind::CannotPerformUnaryOperationForType { operator, bad_type } => {
                write!(f, "Cannot perform '{}' on '{}'", operator, bad_type)?;
            }
            ResolveErrorKind::CannotPerformBinaryOperationForType { operator, bad_type } => {
                write!(f, "Cannot perform '{}' on '{}'", operator, bad_type)?;
            }
            ResolveErrorKind::MismatchingYieldedTypes { got } => {
                let got = got
                    .iter()
                    .unique()
                    .take(5)
                    .map(|type_name| format!("'{}'", type_name))
                    .join(", ");

                match got.len() {
                    0..=4 => write!(f, "Mismatching yielded types - {}", got)?,
                    _ => write!(f, "Mismatching yielded types - {}, ...", got)?,
                }
            }
            ResolveErrorKind::StringTypeNotDefined => {
                write!(f, "String type not defined")?;
            }
            ResolveErrorKind::ExpectedTypeForField {
                structure,
                field_name,
                expected,
            } => {
                write!(
                    f,
                    "Expected value of type '{}' for field '{}' of '{}'",
                    expected, field_name, structure
                )?;
            }
            ResolveErrorKind::CannotAccessMemberOf { bad_type } => {
                write!(f, "Cannot access member of type '{}'", bad_type)?;
            }
            ResolveErrorKind::ExpectedIndexOfType { expected, got } => {
                write!(f, "Expected index of type '{}', got '{}'", expected, got)?;
            }
            ResolveErrorKind::ExpectedTypeForSide { side, operator, expected, got } => {
                write!(f, "Expected '{}' value for {} of '{}', got '{}' value", expected, side, operator, got)?;
            }
            ResolveErrorKind::Other { message } => {
                write!(f, "{}", message)?;
            }
        }

        Ok(())
    }
}
