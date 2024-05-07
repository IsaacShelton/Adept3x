mod ast;
mod expand;
mod lexer;
mod line_splice;
mod parser;
mod token;

use self::expand::{expand_ast, Environment};
use self::lexer::lex;
use self::line_splice::LineSplicer;
use self::parser::parse;

#[derive(Clone, Debug)]
pub enum PreprocessorError {
    UnterminatedMultiLineComment,
    UnterminatedCharacterConstant,
    UnterminatedStringLiteral,
    UnterminatedHeaderName,
    BadEscapeSequence,
    BadEscapedCodepoint,
    ParseError(ParseError),
}

#[derive(Clone, Debug)]
pub enum ParseError {
    ExpectedGroupPart,
    ExpectedIdentifier,
    UnexpectedToken { after: String },
    ExpectedEndif,
    UnrecognizedDirective(String),
    ExpectedDefinitionName,
    ExpectedNewlineAfterDirective,
    UnrecognizedPragmaDirective(String),
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
