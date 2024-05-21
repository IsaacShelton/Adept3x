mod ast;
mod expand;
mod lexer;
mod line_splice;
mod parser;
mod pre_token;

use self::expand::{expand_ast, Environment};
use self::lexer::lex;
use self::line_splice::LineSplicer;
use self::parser::parse;
use self::pre_token::Punctuator;
use std::fmt::Display;
use std::num::NonZeroU32;

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
    pub line: Option<NonZeroU32>,
}

impl PreprocessorError {
    pub fn new(kind: PreprocessorErrorKind, line: Option<NonZeroU32>) -> Self {
        Self { kind, line }
    }
}

impl From<ParseError> for PreprocessorError {
    fn from(value: ParseError) -> Self {
        Self {
            kind: PreprocessorErrorKind::ParseError(value.kind),
            line: value.line,
        }
    }
}

impl Display for PreprocessorError {
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
}

impl PreprocessorErrorKind {
    pub fn at(self, line: Option<NonZeroU32>) -> PreprocessorError {
        PreprocessorError::new(self, line)
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
        }
    }
}

#[derive(Clone, Debug)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub line: Option<NonZeroU32>,
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
    // Initial parsing errors...
    ExpectedGroupPart,
    ExpectedIdentifier,
    UnexpectedToken { after: String },
    ExpectedEndif,
    UnrecognizedDirective(String),
    ExpectedDefinitionName,
    ExpectedNewlineAfterDirective,
    UnrecognizedPragmaDirective(String),
    ExpectedOpenParen,
    ExpectedParameterName,
    ExpectedComma,
    ExpectedCloseParenAfterVarArgs,
    ExpectedPunctuator(Punctuator),
    // Expression parsing errors... (These occur during expansion)
    ExpectedExpression,
    BadInteger,
    ExpectedCloseParen,
    ExpectedColon,
    NotEnoughArguments,
    TooManyArguments,
    ExpectedOpenParenDuringExpansion,
    ExpectedEndOfExpression,
}

impl ParseErrorKind {
    pub fn at(self, line: Option<NonZeroU32>) -> ParseError {
        ParseError::new(self, line)
    }
}

impl Display for ParseErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseErrorKind::ExpectedGroupPart => {
                f.write_str("expected group part during preprocessing")
            }
            ParseErrorKind::ExpectedIdentifier => {
                f.write_str("expected identifier during preprocessing")
            }
            ParseErrorKind::UnexpectedToken { after } => {
                write!(f, "Unexpected token after {} during preprocessing", after)
            }
            ParseErrorKind::ExpectedEndif => f.write_str("unexpected #endif"),
            ParseErrorKind::UnrecognizedDirective(directive) => {
                write!(f, "Unrecognized preprocessing directive #{}", directive)
            }
            ParseErrorKind::ExpectedDefinitionName => {
                f.write_str("Expected definition name during preprocessing")
            }
            ParseErrorKind::ExpectedNewlineAfterDirective => {
                f.write_str("Expected newline after preprocessing directive")
            }
            ParseErrorKind::UnrecognizedPragmaDirective(directive) => {
                write!(f, "Unrecognized pragma directive {}", directive)
            }
            ParseErrorKind::ExpectedOpenParen => f.write_str("Expected '(' during preprocessing"),
            ParseErrorKind::ExpectedParameterName => {
                f.write_str("Expected parameter name during preprocessing")
            }
            ParseErrorKind::ExpectedComma => f.write_str("Expected ',' during preprocessing"),
            ParseErrorKind::ExpectedCloseParenAfterVarArgs => {
                f.write_str("Expected ')' after '...' in macro parameter list")
            }
            ParseErrorKind::ExpectedPunctuator(punctuator) => {
                write!(f, "Expected '{}' during preprocessing", punctuator)
            }
            ParseErrorKind::ExpectedExpression => {
                f.write_str("Expected expression during preprocessing")
            }
            ParseErrorKind::BadInteger => f.write_str("Bad integer during preprocessing"),
            ParseErrorKind::ExpectedCloseParen => f.write_str("Expected ')' during preprocessing"),
            ParseErrorKind::ExpectedColon => f.write_str("Expected ':' during preprocessing"),
            ParseErrorKind::NotEnoughArguments => {
                f.write_str("Not enough arguments to preprocessing macro")
            }
            ParseErrorKind::TooManyArguments => {
                f.write_str("Too many arguments to preprocessing macro")
            }
            ParseErrorKind::ExpectedOpenParenDuringExpansion => {
                f.write_str("Expected '(' during preprocessor macro expansion")
            }
            ParseErrorKind::ExpectedEndOfExpression => {
                f.write_str("Expected end of expression during preprocessing")
            }
        }
    }
}

pub fn preprocess(content: &str) -> Result<Vec<PreToken>, PreprocessorError> {
    let lines = LineSplicer::new(content.chars());
    let mut tokens = lex(lines)?;

    let ast = match parse(tokens.drain(0..)) {
        Ok(ast) => ast,
        Err(err) => return Err(err.into()),
    };

    expand_ast(&ast, Environment::default())
}
