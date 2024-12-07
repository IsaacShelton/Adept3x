use crate::{
    resolve::{PolymorphError, PolymorphErrorKind},
    show::Show,
    source_files::{Source, SourceFiles},
};
use std::fmt::Display;

pub struct LowerError {
    pub kind: LowerErrorKind,
    pub source: Source,
}

impl LowerError {
    pub fn new(kind: LowerErrorKind, source: Source) -> Self {
        Self { kind, source }
    }
}

#[derive(Clone, Debug)]
pub enum LowerErrorKind {
    MustReturnValueOfTypeBeforeExitingFunction {
        return_type: String,
        function: String,
    },
    CannotLowerUnspecializedIntegerLiteral {
        value: String,
    },
    CannotLowerUnspecializedFloatLiteral {
        value: String,
    },
    CannotFit {
        value: String,
        expected_type: String,
    },
    NoSuchEnumMember {
        enum_name: String,
        variant_name: String,
    },
    EnumBackingTypeMustBeInteger {
        enum_name: String,
    },
    PolymorphError(PolymorphErrorKind),
    IncorrectNumberOfTypeArguments,
}

impl From<PolymorphError> for LowerError {
    fn from(value: PolymorphError) -> Self {
        LowerErrorKind::PolymorphError(value.kind).at(value.source)
    }
}

impl LowerErrorKind {
    pub fn at(self, source: Source) -> LowerError {
        LowerError { kind: self, source }
    }
}

impl Show for LowerError {
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

impl Display for LowerErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LowerErrorKind::MustReturnValueOfTypeBeforeExitingFunction {
                return_type,
                function,
            } => {
                write!(
                    f,
                    "Must return a value of type '{}' before exiting function '{}'",
                    return_type, function
                )
            }
            LowerErrorKind::CannotLowerUnspecializedIntegerLiteral { value } => {
                write!(f, "Cannot lower unspecialized integer literal {}", value)
            }
            LowerErrorKind::CannotLowerUnspecializedFloatLiteral { value } => {
                write!(f, "Cannot lower unspecialized float literal {}", value)
            }
            LowerErrorKind::CannotFit {
                value,
                expected_type,
            } => {
                write!(f, "Cannot fit {} into {}", value, expected_type)
            }
            LowerErrorKind::NoSuchEnumMember {
                enum_name,
                variant_name,
            } => {
                write!(f, "No member '{}' of enum '{}'", variant_name, enum_name)
            }
            LowerErrorKind::EnumBackingTypeMustBeInteger { enum_name } => {
                write!(f, "Backing type must be integer for enum '{}'", enum_name)
            }
            LowerErrorKind::IncorrectNumberOfTypeArguments => {
                write!(f, "Incorrect number of type arguments")
            }
            LowerErrorKind::PolymorphError(e) => e.fmt(f),
        }
    }
}
