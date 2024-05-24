use std::{fmt::Display, num::NonZeroU32};

#[derive(Clone, Debug)]
pub struct ParseError {
    kind: ParseErrorKind,
    line: Option<NonZeroU32>,
}

impl ParseError {
    pub fn new(kind: ParseErrorKind, line: Option<NonZeroU32>) -> Self {
        Self { kind, line }
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(line) = self.line {
            write!(f, "error on line {}: ", line)?;
        } else {
            write!(f, "error: ")?;
        }
        self.kind.fmt(f)
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
            ParseErrorKind::CannotReturnFunctionPointerType => f.write_str("Cannot return function pointer type"),
            ParseErrorKind::AutoNotSupportedForReturnType => f.write_str("'auto' not supported for return type"),
            ParseErrorKind::ConstexprNotSupportedForReturnType => f.write_str("'constexpr' not supported for return type"),
            ParseErrorKind::InvalidType => f.write_str("Invalid type"),
            ParseErrorKind::Misc(message) => f.write_str(message),
        }
    }
}
