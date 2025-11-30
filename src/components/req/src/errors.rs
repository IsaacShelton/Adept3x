use std::fmt::Display;
use thiserror::Error;
use top_n::TopN;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Errs {
    top_n: TopN<Err>,
}

impl Default for Errs {
    fn default() -> Self {
        Self {
            top_n: TopN::new(1),
        }
    }
}

impl From<Err> for Errs {
    fn from(value: Err) -> Self {
        Self {
            top_n: TopN::from_iter(1, std::iter::once(value), |a, b| a.cmp(b)),
        }
    }
}

impl Errs {
    pub fn push(&mut self, error: Err) -> &mut Self {
        self.top_n.push(error, |a, b| a.cmp(b));
        self
    }
}

impl Display for Errs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<err msg>")
    }
}

#[derive(Clone, Debug, Error, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Err {
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
}
