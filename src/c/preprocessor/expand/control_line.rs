use super::{depleted::Depleted, embed::expand_embed, include::expand_include, Environment};
use crate::{
    c::preprocessor::{
        ast::{ControlLine, ControlLineKind, Define},
        error::PreprocessorErrorKind,
        pre_token::PreToken,
        PreprocessorError,
    },
    source_files::Source,
};
use itertools::Itertools;

pub fn expand_control_line(
    control_line: &ControlLine,
    environment: &mut Environment,
    depleted: &mut Depleted,
) -> Result<Vec<PreToken>, PreprocessorError> {
    let source = control_line.source;

    match &control_line.kind {
        ControlLineKind::Include(files) => expand_include(files, environment, depleted, source),
        ControlLineKind::Embed(options) => expand_embed(options, environment, depleted),
        ControlLineKind::Define(define) => expand_define(define, environment),
        ControlLineKind::Undef(identifier) => expand_undef(identifier, environment),
        ControlLineKind::Line(_) => Ok(vec![]),
        ControlLineKind::Error(tokens) => expand_error(tokens, source),
        ControlLineKind::Warning(tokens) => expand_warning(tokens, source),
        ControlLineKind::Pragma(tokens) => expand_pragma(tokens, source),
    }
}

fn expand_define(
    define: &Define,
    environment: &mut Environment,
) -> Result<Vec<PreToken>, PreprocessorError> {
    environment.add_define(define.clone());
    Ok(vec![])
}

fn expand_undef(
    name: &str,
    environment: &mut Environment,
) -> Result<Vec<PreToken>, PreprocessorError> {
    environment.remove_define(name);
    Ok(vec![])
}

fn expand_error(tokens: &[PreToken], source: Source) -> Result<Vec<PreToken>, PreprocessorError> {
    Err(PreprocessorErrorKind::ErrorDirective(
        tokens.iter().map(|token| token.to_string()).join(" "),
    )
    .at(source))
}

fn expand_warning(tokens: &[PreToken], source: Source) -> Result<Vec<PreToken>, PreprocessorError> {
    let warning = tokens.iter().map(|token| token.to_string()).join(" ");
    eprintln!("#warning on line {}: {}", source.location.line, warning);
    Ok(vec![])
}

fn expand_pragma(_tokens: &[PreToken], source: Source) -> Result<Vec<PreToken>, PreprocessorError> {
    Err(PreprocessorErrorKind::UnsupportedPragma.at(source))
}
