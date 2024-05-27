use crate::{ast::Source, show::Show, source_file_cache::SourceFileCache};
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
    }
}

impl LowerErrorKind {
    pub fn at(self, source: Source) -> LowerError {
        LowerError { kind: self, source }
    }
}

impl Show for LowerError {
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
            LowerErrorKind::CannotFit { value, expected_type } => {
                write!(f, "Cannot fit {} into {}", value, expected_type)
            }
        }
    }
}
