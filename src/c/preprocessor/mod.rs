mod ast;
mod expand;
mod lexer;
mod line_splice;
mod parser;
mod pre_token;

use crate::ast::Source;
use crate::show::Show;
use crate::source_file_cache::SourceFileCache;
use crate::text::Text;

use self::expand::{expand_ast, Environment};
use self::lexer::Lexer;
use self::parser::{parse, ParseError, ParseErrorKind};
use crate::inflow::IntoInflow;
use std::fmt::Display;

/*
   Missing features:
   - __has_include
   - __has_embed
   - __has_c_attribute
   - #embed (and its options)
   - #pragma STDC (all of its options)
   - __FILE__
   - __LINE__
   - __DATE__
   - etc.
*/

pub use self::pre_token::{PreToken, PreTokenKind};

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
                f.write_str("Unterminated multi-line comment")
            }
            PreprocessorErrorKind::UnterminatedCharacterConstant => {
                f.write_str("Unterminated character constant")
            }
            PreprocessorErrorKind::UnterminatedStringLiteral => {
                f.write_str("Unterminated string literal")
            }
            PreprocessorErrorKind::UnterminatedHeaderName => {
                f.write_str("Unterminated header name")
            }
            PreprocessorErrorKind::BadEscapeSequence => f.write_str("Bad escape sequence"),
            PreprocessorErrorKind::BadEscapedCodepoint => f.write_str("Bad escaped codepoint"),
            PreprocessorErrorKind::ParseError(err) => err.fmt(f),
            PreprocessorErrorKind::BadInclude => f.write_str("Bad #include"),
            PreprocessorErrorKind::ErrorDirective(message) => write!(f, "{}", message),
            PreprocessorErrorKind::UnsupportedPragma => f.write_str("Unsupported pragma"),
            PreprocessorErrorKind::CannotConcatTokens => {
                f.write_str("Cannot concatenate those preprocessor tokens")
            }
            PreprocessorErrorKind::ExpectedEof => f.write_str("Expected end-of-file"),
        }
    }
}

pub fn preprocess(text: impl Text) -> Result<(Vec<PreToken>, Source), PreprocessorError> {
    let lexer = Lexer::new(text);

    let ast = match parse(lexer.into_inflow()) {
        Ok(ast) => ast,
        Err(err) => return Err(err.into()),
    };

    Ok((expand_ast(&ast, Environment::default())?, ast.eof))
}
