use super::parser::{ParseError, ParseErrorKind};
use crate::{
    show::Show,
    source_files::{Source, SourceFiles},
};
use std::fmt::Display;

#[derive(Clone, Debug)]
pub struct PreprocessorError {
    pub kind: PreprocessorErrorKind,
    pub source: Source,
}

impl PreprocessorError {
    pub fn new(kind: PreprocessorErrorKind, source: Source) -> Self {
        Self { kind, source }
    }
}

impl From<ParseError> for PreprocessorError {
    fn from(value: ParseError) -> Self {
        Self {
            kind: PreprocessorErrorKind::ParseError(value.kind),
            source: value.source,
        }
    }
}

impl Show for PreprocessorError {
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

#[derive(Clone, Debug)]
pub enum PreprocessorErrorKind {
    UnterminatedMultiLineComment,
    UnterminatedCharacterConstant,
    UnterminatedStringLiteral,
    UnterminatedHeaderName,
    BadEscapeSequence,
    BadEscapedCodepoint,
    ParseError(ParseErrorKind),
    BadInclude,
    ErrorDirective(String),
    UnsupportedPragma,
    CannotConcatTokens,
    ExpectedEof,
}

impl PreprocessorErrorKind {
    pub fn at(self, source: Source) -> PreprocessorError {
        PreprocessorError::new(self, source)
    }
}

impl Display for PreprocessorErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PreprocessorErrorKind::UnterminatedMultiLineComment => {
                write!(f, "Unterminated multi-line comment")
            }
            PreprocessorErrorKind::UnterminatedCharacterConstant => {
                write!(f, "Unterminated character constant")
            }
            PreprocessorErrorKind::UnterminatedStringLiteral => {
                write!(f, "Unterminated string literal")
            }
            PreprocessorErrorKind::UnterminatedHeaderName => {
                write!(f, "Unterminated header name")
            }
            PreprocessorErrorKind::BadEscapeSequence => write!(f, "Bad escape sequence"),
            PreprocessorErrorKind::BadEscapedCodepoint => write!(f, "Bad escaped codepoint"),
            PreprocessorErrorKind::ParseError(err) => write!(f, "{err}"),
            PreprocessorErrorKind::BadInclude => write!(f, "Bad #include"),
            PreprocessorErrorKind::ErrorDirective(message) => write!(f, "{message}"),
            PreprocessorErrorKind::UnsupportedPragma => write!(f, "Unsupported pragma"),
            PreprocessorErrorKind::CannotConcatTokens => {
                write!(f, "Cannot concatenate those preprocessor tokens")
            }
            PreprocessorErrorKind::ExpectedEof => write!(f, "Expected end-of-file"),
        }
    }
}
