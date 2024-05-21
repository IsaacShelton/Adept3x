use std::{fmt::Display, num::NonZeroU32};

#[derive(Clone, Debug)]
pub struct ParseError {
    kind: ParseErrorKind,
    line: Option<NonZeroU32>,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(line) = self.line {
            write!(f, "error on line {}: ", line)?;
        } else {
            write!(f, "error: ")?;
        }
        self.fmt(f)
    }
}

#[derive(Clone, Debug)]
pub enum ParseErrorKind {}

impl Display for ParseErrorKind {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            _ => todo!(),
        }
    }
}
