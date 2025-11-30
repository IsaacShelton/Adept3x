use thiserror::Error;

#[derive(Clone, Debug, Error, Hash, PartialEq, Eq, PartialOrd, Ord)]
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
}
