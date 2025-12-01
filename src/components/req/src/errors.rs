use derive_more::IsVariant;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
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
}
