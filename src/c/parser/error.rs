use crate::{ast::Source, show::Show, source_file_cache::SourceFileCache};
use std::fmt::Display;

#[derive(Clone, Debug)]
pub struct ParseError {
    kind: ParseErrorKind,
    source: Source,
}

impl ParseError {
    pub fn new(kind: ParseErrorKind, source: Source) -> Self {
        Self { kind, source }
    }
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
            self.kind,
        )
    }
}

#[derive(Clone, Debug)]
pub enum ParseErrorKind {
    ExpectedDeclaration,
    CannotReturnFunctionPointerType,
    AutoNotSupportedForReturnType,
    ConstexprNotSupportedForReturnType,
    InvalidType,
    Misc(&'static str),
}

impl Display for ParseErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseErrorKind::ExpectedDeclaration => f.write_str("Expected declaration"),
            ParseErrorKind::CannotReturnFunctionPointerType => {
                f.write_str("Cannot return function pointer type")
            }
            ParseErrorKind::AutoNotSupportedForReturnType => {
                f.write_str("'auto' not supported for return type")
            }
            ParseErrorKind::ConstexprNotSupportedForReturnType => {
                f.write_str("'constexpr' not supported for return type")
            }
            ParseErrorKind::InvalidType => f.write_str("Invalid type"),
            ParseErrorKind::Misc(message) => f.write_str(message),
        }
    }
}
