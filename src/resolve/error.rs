use crate::{ast::Source, show::Show, source_file_cache::SourceFileCache};
use itertools::Itertools;
use std::fmt::Display;

pub struct ResolveError {
    pub kind: ResolveErrorKind,
    pub source: Source,
}

impl ResolveError {
    pub fn new(kind: ResolveErrorKind, source: Source) -> Self {
        Self { kind, source }
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
    ArraySizeTooLarge,
    MultipleDefinitionsOfTypeNamed {
        name: String,
    },
    Other {
        message: String,
    },
}

impl ResolveErrorKind {
    pub fn at(self, source: Source) -> ResolveError {
        ResolveError { kind: self, source }
    }
}

impl Show for ResolveError {
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

impl Display for ResolveErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
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
            ResolveErrorKind::ExpectedTypeForSide {
                side,
                operator,
                expected,
                got,
            } => {
                write!(
                    f,
                    "Expected '{}' value for {} of '{}', got '{}' value",
                    expected, side, operator, got
                )?;
            }
            ResolveErrorKind::ArraySizeTooLarge => {
                write!(f, "Array size is too large")?;
            }
            ResolveErrorKind::MultipleDefinitionsOfTypeNamed { name } => {
                write!(f, "Multiple definitions of type named '{}'", name)?;
            },
            ResolveErrorKind::Other { message } => {
                write!(f, "{}", message)?;
            }
        }

        Ok(())
    }
}
