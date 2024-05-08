mod control_line;
mod depleted;
mod environment;
mod include;
mod region;
mod embed;

use self::{control_line::expand_control_line, region::expand_region};
use super::{
    ast::{
        ElifGroup, Group, IfDefKind, IfDefLike, IfGroup, IfLike, IfSection, PreprocessorAst,
        TextLine,
    },
    pre_token::PreToken,
    PreprocessorError,
};
use crate::c::preprocessor::ast::GroupPart;
use depleted::Depleted;
use itertools::Itertools;

pub use self::environment::Environment;

#[derive(Clone, Debug)]
pub enum Token {
    PreToken(PreToken),
}

pub fn expand_ast(
    ast: &PreprocessorAst,
    environment: Environment,
) -> Result<Vec<Token>, PreprocessorError> {
    let mut environment = environment;
    let mut depleted = Depleted::new();
    expand_group(&ast.group, &mut environment, &mut depleted)
}

fn expand_group(
    group: &Group,
    environment: &mut Environment,
    depleted: &mut Depleted,
) -> Result<Vec<Token>, PreprocessorError> {
    let mut tokens = Vec::with_capacity(1024);

    for part in group.parts.iter() {
        match part {
            GroupPart::IfSection(if_section) => {
                tokens.append(&mut expand_if_section(if_section, environment, depleted)?)
            }
            GroupPart::ControlLine(control_line) => {
                tokens.append(&mut expand_control_line(
                    control_line,
                    environment,
                    depleted,
                )?);
            }
            GroupPart::TextLine(text_line) => {
                tokens.append(&mut expand_text_line(text_line, environment, depleted)?);
            }
            GroupPart::HashNonDirective => (), // Ignored during expansion
        }
    }

    Ok(tokens)
}

fn expand_if_like(
    if_like: &IfLike,
    environment: &mut Environment,
    depleted: &mut Depleted,
) -> Result<Option<Vec<Token>>, PreprocessorError> {
    if if_like.constant_expression.is_true() {
        Ok(Some(expand_group(&if_like.group, environment, depleted)?))
    } else {
        Ok(None)
    }
}

fn expand_if_def_like(
    if_def_like: &IfDefLike,
    environment: &mut Environment,
    depleted: &mut Depleted,
) -> Result<Option<Vec<Token>>, PreprocessorError> {
    let invert = match if_def_like.kind {
        IfDefKind::Defined => false,
        IfDefKind::NotDefined => true,
    };

    let defined = environment
        .find_defines_of_name(&if_def_like.identifier)
        .is_some();

    Ok(if defined ^ invert {
        Some(expand_group(&if_def_like.group, environment, depleted)?)
    } else {
        None
    })
}

fn expand_if_section(
    if_section: &IfSection,
    environment: &mut Environment,
    depleted: &mut Depleted,
) -> Result<Vec<Token>, PreprocessorError> {
    if let Some(tokens) = match &if_section.if_group {
        IfGroup::IfLike(if_like) => expand_if_like(if_like, environment, depleted)?,
        IfGroup::IfDefLike(if_def_like) => expand_if_def_like(if_def_like, environment, depleted)?,
    } {
        return Ok(tokens);
    }

    for elif in if_section.elif_groups.iter() {
        if let Some(tokens) = match elif {
            ElifGroup::Elif(if_like) => expand_if_like(if_like, environment, depleted)?,
            ElifGroup::ElifDef(if_def_like) => {
                expand_if_def_like(if_def_like, environment, depleted)?
            }
        } {
            return Ok(tokens);
        }
    }

    if let Some(else_group) = &if_section.else_group {
        expand_group(&else_group, environment, depleted)
    } else {
        Ok(vec![])
    }
}

fn expand_text_line(
    text_line: &TextLine,
    environment: &Environment,
    depleted: &mut Depleted,
) -> Result<Vec<Token>, PreprocessorError> {
    tokenize(&expand_region(&text_line.content, environment, depleted)?)
}

fn tokenize(tokens: &[PreToken]) -> Result<Vec<Token>, PreprocessorError> {
    Ok(tokens
        .iter()
        .map(|token| Token::PreToken(token.clone()))
        .collect_vec())
}
