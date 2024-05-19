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

pub fn preprocess(content: &str) -> Result<String, PreprocessorError> {
    let lines = LineSplicer::new(content.chars());
    let mut tokens = lex(lines)?;

    let ast = match parse(tokens.drain(0..)) {
        Ok(ast) => ast,
        Err(err) => return Err(err.into()),
    };

    let expanded = expand_ast(&ast, Environment::default())?;

    Ok(format!("{:#?}", expanded))
}
