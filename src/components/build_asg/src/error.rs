use super::func_haystack::FindFunctionError;
use asg::{PolymorphError, PolymorphErrorKind};
use diagnostics::Show;
use itertools::Itertools;
use source_files::{Source, SourceFiles};
use std::fmt::Display;

#[derive(Clone, Debug)]
pub struct ResolveError {
    pub kind: ResolveErrorKind,
    pub source: Source,
}

impl ResolveError {
    pub fn new(kind: ResolveErrorKind, source: Source) -> Self {
        Self { kind, source }
    }

    pub fn other(message: impl ToString, source: Source) -> Self {
        Self {
            kind: ResolveErrorKind::Other {
                message: message.to_string(),
            },
            source,
        }
    }
}

impl From<UnaliasError> for ResolveErrorKind {
    fn from(value: UnaliasError) -> Self {
        Self::UnaliasError(value)
    }
}

impl From<PolymorphError> for ResolveError {
    fn from(value: PolymorphError) -> Self {
        ResolveErrorKind::from(value.kind).at(value.source)
    }
}

impl From<PolymorphErrorKind> for ResolveErrorKind {
    fn from(value: PolymorphErrorKind) -> Self {
        Self::PolymorphError(value)
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
        signature: String,
        reason: FindFunctionError,
        almost_matches: Vec<String>,
    },
    UndeclaredVariable {
        name: String,
    },
    UndeclaredType {
        name: String,
    },
    AmbiguousType {
        name: String,
    },
    AmbiguousGlobal {
        name: String,
    },
    AmbiguousHelperExpr {
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
    CannotCreateStructLiteralForNonStructure {
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
    MultipleDefinesNamed {
        name: String,
    },
    RecursiveTypeAlias {
        name: String,
    },
    OutOfFields,
    FieldSpecifiedMoreThanOnce {
        struct_name: String,
        field_name: String,
    },
    MustInitializeVariable {
        name: String,
    },
    FunctionMustReturnType {
        of: String,
        func_name: String,
    },
    DivideByZero,
    ModuloByZero,
    ShiftByNegative,
    ShiftTooLarge,
    CannotPerformOnUnspecializedInteger {
        operation: String,
    },
    StaticMemberOfTypeDoesNotExist {
        ty: String,
        member: String,
    },
    AmbiguousSymbol {
        name: String,
    },
    UndeterminedCharacterLiteral,
    UnaliasError(UnaliasError),
    CannotDeclareVariableOutsideFunction,
    CannotReturnOutsideFunction,
    CannotAssignVariableOutsideFunction,
    PolymorphError(PolymorphErrorKind),
    UndeclaredTrait(String),
    CannotUseOperator {
        operator: String,
        on_type: String,
    },
    ConstraintsNotSatisfiedForType {
        name: String,
    },
    TypeIsNotATrait {
        name: String,
    },
    DuplicateTypeName {
        name: String,
    },
    DuplicateImplementationName {
        name: String,
    },
    CannotCreateOutOfRangeFloat,
    FailedToConformArgumentToDefaultValue,
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
    fn show(&self, w: &mut dyn std::fmt::Write, source_files: &SourceFiles) -> std::fmt::Result {
        write!(
            w,
            "{}:{}:{}: error: {}",
            source_files.get(self.source.key).filename(),
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
            ResolveErrorKind::FailedToFindFunction {
                signature,
                reason,
                almost_matches,
            } => {
                match reason {
                    FindFunctionError::NotDefined => {
                        write!(f, "Failed to find function '{}'", signature)?
                    }
                    FindFunctionError::Ambiguous => {
                        write!(f, "Multiple possibilities for function '{}'", signature)?
                    }
                }

                if !almost_matches.is_empty() {
                    match reason {
                        FindFunctionError::NotDefined => {
                            write!(f, "\n    Did you mean?")?;
                        }
                        FindFunctionError::Ambiguous => {
                            write!(f, "\n    Possibilities include:")?;
                        }
                    }

                    for almost_match in almost_matches {
                        write!(f, "\n    {}", almost_match)?;
                    }
                } else {
                    write!(
                        f,
                        "\n    No available functions have that name, is it public?"
                    )?;
                }
            }
            ResolveErrorKind::UndeclaredVariable { name } => {
                write!(f, "Undeclared variable '{}'", name)?;
            }
            ResolveErrorKind::UndeclaredType { name } => {
                write!(f, "Undeclared type '{}'", name)?;
            }
            ResolveErrorKind::AmbiguousType { name } => {
                write!(f, "Ambiguous type '{}'", name)?;
            }
            ResolveErrorKind::AmbiguousGlobal { name } => {
                write!(f, "Ambiguous global variable '{}'", name)?;
            }
            ResolveErrorKind::AmbiguousHelperExpr { name } => {
                write!(f, "Ambiguous define '{}'", name)?;
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
            ResolveErrorKind::CannotCreateStructLiteralForNonStructure { bad_type } => {
                write!(
                    f,
                    "Cannot create struct literal for non-structure '{}'",
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
            }
            ResolveErrorKind::MultipleDefinesNamed { name } => {
                write!(f, "Multiple defines named '{}'", name)?;
            }
            ResolveErrorKind::RecursiveTypeAlias { name } => {
                write!(f, "Recursive type alias '{}'", name)?;
            }
            ResolveErrorKind::OutOfFields => {
                write!(f, "Out of fields to populate for struct literal")?;
            }
            ResolveErrorKind::FieldSpecifiedMoreThanOnce {
                struct_name,
                field_name,
            } => {
                write!(
                    f,
                    "'{}' is specified more than once for '{}' literal",
                    field_name, struct_name
                )?;
            }
            ResolveErrorKind::MustInitializeVariable { name } => {
                write!(f, "Must provide initial value for variable '{}'", name)?;
            }

            ResolveErrorKind::FunctionMustReturnType { of, func_name } => {
                write!(f, "Function '{}' must return '{}'", func_name, of)?;
            }
            ResolveErrorKind::DivideByZero => {
                write!(f, "Cannot divide by zero")?;
            }
            ResolveErrorKind::ModuloByZero => {
                write!(f, "Cannot modulo by zero")?;
            }
            ResolveErrorKind::ShiftByNegative => {
                write!(f, "Cannot shift by negative")?;
            }
            ResolveErrorKind::ShiftTooLarge => {
                write!(f, "Cannot shift by that large amount")?;
            }
            ResolveErrorKind::CannotPerformOnUnspecializedInteger { operation } => {
                write!(f, "Cannot {operation} unspecialized integers")?;
            }
            ResolveErrorKind::StaticMemberOfTypeDoesNotExist { ty, member } => {
                write!(f, "Static member '{member}' does not exist on type '{ty}'")?;
            }
            ResolveErrorKind::AmbiguousSymbol { name } => {
                write!(f, "Ambiguous symbol '{name}'")?;
            }
            ResolveErrorKind::UndeterminedCharacterLiteral => {
                write!(
                    f,
                    "Undetermined character literal, consider using c'A' if you want a 'char'"
                )?;
            }
            ResolveErrorKind::UnaliasError(details) => match details {
                UnaliasError::MaxDepthExceeded => write!(f, "Maximum type alias depth exceeded")?,
                UnaliasError::SelfReferentialTypeAlias(type_alias_name) => {
                    write!(f, "Type alias '{}' is self-referential", type_alias_name)?
                }
                UnaliasError::IncorrectNumberOfTypeArgsForAlias(type_alias_name) => write!(
                    f,
                    "Incorrect number of type arguments for type alias '{}'",
                    type_alias_name
                )?,
                UnaliasError::PolymorphError(e) => {
                    e.fmt(f)?;
                }
            },
            ResolveErrorKind::CannotDeclareVariableOutsideFunction => {
                write!(f, "Cannot declare variable outside of function")?;
            }
            ResolveErrorKind::CannotReturnOutsideFunction => {
                write!(f, "Cannot return outside of function")?;
            }
            ResolveErrorKind::CannotAssignVariableOutsideFunction => {
                write!(f, "Cannot assign variable outside of function")?;
            }
            ResolveErrorKind::PolymorphError(e) => {
                e.fmt(f)?;
            }
            ResolveErrorKind::UndeclaredTrait(trait_name) => {
                write!(f, "Undeclared trait '{}'", trait_name)?;
            }
            ResolveErrorKind::CannotUseOperator { operator, on_type } => {
                write!(
                    f,
                    "Cannot use operator '{}' on type '{}'",
                    operator, on_type
                )?;
            }
            ResolveErrorKind::ConstraintsNotSatisfiedForType { name } => {
                write!(f, "Constraints not satisfied for type '{}'", name)?;
            }
            ResolveErrorKind::TypeIsNotATrait { name } => {
                write!(f, "Type '{}' is not a trait", name)?;
            }
            ResolveErrorKind::DuplicateTypeName { name } => {
                write!(f, "Duplicate type name '{}'", name)?;
            }
            ResolveErrorKind::DuplicateImplementationName { name } => {
                write!(f, "Duplicate implementation name '{}'", name)?;
            }
            ResolveErrorKind::CannotCreateOutOfRangeFloat => {
                write!(f, "Cannot create out-of-range floating-point number")?;
            }
            ResolveErrorKind::FailedToConformArgumentToDefaultValue => {
                write!(f, "Failed to conform argument to default value")?;
            }
            ResolveErrorKind::Other { message } => {
                write!(f, "{}", message)?;
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub enum UnaliasError {
    MaxDepthExceeded,
    SelfReferentialTypeAlias(String),
    IncorrectNumberOfTypeArgsForAlias(String),
    PolymorphError(PolymorphErrorKind),
}
