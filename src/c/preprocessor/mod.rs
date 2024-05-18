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

/*
   Missing features:
   - ## concatenating
   - __has_include
   - __has_embed
   - #embed (and its options)
   - #pragma STDC (all of its options)
   - __FILE__
   - __LINE__
   - etc.
*/

#[derive(Clone, Debug)]
pub enum PreprocessorError {
    UnterminatedMultiLineComment,
    UnterminatedCharacterConstant,
    UnterminatedStringLiteral,
    UnterminatedHeaderName,
    BadEscapeSequence,
    BadEscapedCodepoint,
    ParseError(ParseError),
    BadInclude,
    ErrorDirective(String),
    UnsupportedPragma,
}

#[derive(Clone, Debug)]
pub enum ParseError {
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
    // Expression parsing errors... (These occur during expansion)
    ExpectedExpression,
    BadInteger,
    ExpectedCloseParen,
    ExpectedColon,
    NotEnoughArguments,
    TooManyArguments,
    ExpectedOpenParenDuringExpansion,
}

pub fn preprocess(content: &str) -> Result<String, PreprocessorError> {
    let lines = LineSplicer::new(content.chars());
    let mut tokens = lex(lines)?;

    let ast = match parse(tokens.drain(0..)) {
        Ok(ast) => ast,
        Err(err) => return Err(PreprocessorError::ParseError(err)),
    };

    let expanded = expand_ast(&ast, Environment::default())?;

    Ok(format!("{:#?}", expanded))
}
