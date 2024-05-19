use std::num::NonZeroU32;

use super::{depleted::Depleted, embed::expand_embed, include::expand_include, Environment};
use crate::c::preprocessor::{
    ast::{ControlLine, ControlLineKind, Define},
    pre_token::PreToken,
    PreprocessorError, PreprocessorErrorKind,
};
use itertools::Itertools;

pub fn expand_control_line(
    control_line: &ControlLine,
    environment: &mut Environment,
    depleted: &mut Depleted,
) -> Result<Vec<PreToken>, PreprocessorError> {
    let line = control_line.line;

    match &control_line.kind {
        ControlLineKind::Include(files) => expand_include(files, environment, depleted, line),
        ControlLineKind::Embed(options) => expand_embed(options, environment, depleted),
        ControlLineKind::Define(define) => expand_define(define, environment),
        ControlLineKind::Undef(identifier) => expand_undef(identifier, environment),
        ControlLineKind::Line(_) => Ok(vec![]),
        ControlLineKind::Error(tokens) => expand_error(tokens, line),
        ControlLineKind::Warning(tokens) => expand_warning(tokens, line),
        ControlLineKind::Pragma(tokens) => expand_pragma(tokens, line),
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

fn expand_error(
    tokens: &[PreToken],
    line: Option<NonZeroU32>,
) -> Result<Vec<PreToken>, PreprocessorError> {
    Err(PreprocessorErrorKind::ErrorDirective(
        tokens.iter().map(|token| token.to_string()).join(" "),
    )
    .at(line))
}

fn expand_warning(
    tokens: &[PreToken],
    line: Option<NonZeroU32>,
) -> Result<Vec<PreToken>, PreprocessorError> {
    let warning = tokens.iter().map(|token| token.to_string()).join(" ");

    if let Some(line) = line {
        eprintln!("#warning on line {}: {}", line, warning);
    } else {
        eprintln!("#warning: {}", warning);
    }

    Ok(vec![])
}

fn expand_pragma(
    _tokens: &[PreToken],
    line: Option<NonZeroU32>,
) -> Result<Vec<PreToken>, PreprocessorError> {
    Err(PreprocessorErrorKind::UnsupportedPragma.at(line))
}
