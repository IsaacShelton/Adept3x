mod ast;
mod lexer;
mod line_splice;
mod parser;
mod token;

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
}

pub fn preprocess(content: &str) -> Result<String, PreprocessorError> {
    let lines = LineSplicer::new(content.chars());
    let tokens = lex(lines)?;
    let _ast = parse(&tokens);

    // macro_expansion();

    Ok(format!("{:?}", tokens))
}
