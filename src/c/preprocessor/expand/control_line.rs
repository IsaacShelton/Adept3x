use super::{depleted::Depleted, embed::expand_embed, include::expand_include, Environment, Token};
use crate::c::preprocessor::{
    ast::{ControlLine, Define},
    pre_token::PreToken,
    PreprocessorError,
};
use itertools::Itertools;

pub fn expand_control_line(
    control_line: &ControlLine,
    environment: &mut Environment,
    depleted: &mut Depleted,
) -> Result<Vec<Token>, PreprocessorError> {
    match control_line {
        ControlLine::Include(files) => expand_include(files, environment, depleted),
        ControlLine::Embed(options) => expand_embed(options, environment, depleted),
        ControlLine::Define(define) => expand_define(define, environment),
        ControlLine::Undef(identifier) => expand_undef(identifier, environment),
        ControlLine::Line(_) => Ok(vec![]),
        ControlLine::Error(tokens) => expand_error(tokens),
        ControlLine::Warning(tokens) => expand_warning(tokens),
        ControlLine::Pragma(tokens) => expand_pragma(tokens),
    }
}

fn expand_define(
    define: &Define,
    environment: &mut Environment,
) -> Result<Vec<Token>, PreprocessorError> {
    environment.add_define(define);
    Ok(vec![])
}

fn expand_undef(
    name: &str,
    environment: &mut Environment,
) -> Result<Vec<Token>, PreprocessorError> {
    environment.remove_define(name);
    Ok(vec![])
}

fn expand_error(tokens: &[PreToken]) -> Result<Vec<Token>, PreprocessorError> {
    Err(PreprocessorError::ErrorDirective(
        tokens.iter().map(|token| token.to_string()).join(" "),
    ))
}

fn expand_warning(tokens: &[PreToken]) -> Result<Vec<Token>, PreprocessorError> {
    eprintln!(
        "#warning: {}",
        tokens.iter().map(|token| token.to_string()).join(" ")
    );
    Ok(vec![])
}

fn expand_pragma(_tokens: &[PreToken]) -> Result<Vec<Token>, PreprocessorError> {
    Err(PreprocessorError::UnsupportedPragma)
}
