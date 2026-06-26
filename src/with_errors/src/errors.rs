use canonical::Canonical;
use derive_more::IsVariant;
use serde::{Deserialize, Serialize};
use std::{
    fmt::Display,
    num::NonZeroU32,
    path::{Path, PathBuf},
    sync::Arc,
};
use thiserror::Error;

#[derive(
    Clone, Debug, Error, Hash, PartialEq, Eq, PartialOrd, Ord, IsVariant, Serialize, Deserialize,
)]
pub enum Error {
    #[error("Missing project file `adept.build`")]
    MissingProjectFile,
    #[error("`adept.build` must be a text file")]
    ProjectFileMustBeText,
    #[error("Failed to open `adept.build`")]
    FailedToOpenProjectFile,
    #[error("Expected char `{0}`")]
    ExpectedChar(char),
    #[error("Invalid syntax in `adept.build`")]
    InvalidProjectConfigSyntax,
    #[error(
        "Missing root file for `adept.build`, e.g. `{{ adept: \"3.0\", main: \"main.adept\" }}`"
    )]
    MissingRootFileInProjectConfig,
    #[error("Unsupported Adept version in `adept.build`, try `{{ adept: \"3.0\" }}`")]
    UnsupportedAdeptVersion,
    #[error("Invalid option `{0}` in `adept.build`")]
    InvalidProjectConfigOption(Arc<str>),
    #[error("Failed to get canonical path for `{0}`")]
    FailedToCanonicalize(Arc<Path>),
    #[error("Failed to open file `{0}`")]
    FailedToOpenFile(Arc<Canonical<PathBuf>>),
    #[error("Missing main function")]
    MissingMainFunction,
    #[error("Incorrect signature for main function")]
    IncorrectSignatureForMainFunction,
    #[error("{0}")]
    TypingError(String),
    #[error("{0}")]
    SyntaxError(String),
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ErrorAt {
    pub error: Error,
    pub filename: Option<Box<str>>,
    pub line: Option<NonZeroU32>,
    pub column: Option<NonZeroU32>,
}

impl From<Error> for ErrorAt {
    fn from(error: Error) -> Self {
        Self::new(error)
    }
}

impl ErrorAt {
    pub fn new(error: Error) -> Self {
        Self {
            error,
            filename: None,
            line: None,
            column: None,
        }
    }

    pub fn new_typing(message: String) -> Self {
        Self {
            error: Error::TypingError(message),
            filename: None,
            line: None,
            column: None,
        }
    }

    pub fn new_at(error: Error, line_index: u32, column_index: u32) -> Self {
        Self {
            error,
            filename: None,
            line: Some(NonZeroU32::new(line_index.saturating_add(1)).unwrap()),
            column: Some(NonZeroU32::new(column_index.saturating_add(1)).unwrap()),
        }
    }

    pub fn with_filename(mut self, filename: Box<str>) -> Self {
        self.filename = Some(filename.into());
        self
    }

    pub fn display<'a>(&'a self, prefix: Option<&'static str>) -> ErrorAtDisplay<'a> {
        ErrorAtDisplay {
            error_at: self,
            prefix,
        }
    }
}

pub struct ErrorAtDisplay<'a> {
    error_at: &'a ErrorAt,
    prefix: Option<&'static str>,
}

impl<'a> Display for ErrorAtDisplay<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(filename) = &self.error_at.filename {
            write!(f, "{filename}:")?;
        } else if self.error_at.line.is_some() || self.error_at.column.is_some() {
            write!(f, "<filename>:")?;
        }

        if let Some(line) = self.error_at.line {
            write!(f, "{line}:")?;
        }
        if let Some(column) = self.error_at.column {
            write!(f, "{column}:")?;
        }

        if self.error_at.line.is_some() || self.error_at.column.is_some() {
            write!(f, " ")?;
        }

        if let Some(prefix) = self.prefix {
            write!(f, "{}: ", prefix)?;
        }

        write!(f, "{}", self.error_at.error)
    }
}
