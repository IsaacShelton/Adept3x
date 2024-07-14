mod control_line;
mod depleted;
mod embed;
mod environment;
mod expr;
mod include;
mod region;

use std::collections::HashMap;

use self::{control_line::expand_control_line, expr::ExprParser, region::expand_region};
use super::{
    ast::{
        Define, DefineKind, ElifGroup, Group, IfDefKind, IfDefLike, IfGroup, IfLike, IfSection,
        PreprocessorAst, TextLine,
    },
    pre_token::PreToken,
    PreprocessorError,
};
use crate::c::preprocessor::ast::GroupPart;
use depleted::Depleted;

pub use self::environment::Environment;

#[derive(Clone, Debug)]
pub enum Token {
    PreToken(PreToken),
}

pub fn expand_ast(
    ast: &PreprocessorAst,
    environment: Environment,
) -> Result<(Vec<PreToken>, HashMap<String, Define>), PreprocessorError> {
    let mut environment = environment;
    let mut depleted = Depleted::new();
    let pre_tokens = expand_group(&ast.group, &mut environment, &mut depleted)?;

    let document = expand_region(&pre_tokens, &environment, &mut depleted)?;

    // Assemble preprocessed #define object macros
    let mut defines = HashMap::<String, Define>::with_capacity(environment.defines.len());
    for (define_name, define) in environment.defines.iter() {
        match &define.kind {
            DefineKind::ObjectMacro(replacement, placeholder_affinity) => {
                let expanded = expand_region(replacement, &environment, &mut depleted)?;
                defines.insert(
                    define_name.clone(),
                    Define {
                        name: define_name.clone(),
                        kind: DefineKind::ObjectMacro(expanded, placeholder_affinity.clone()),
                        source: define.source,
                    },
                );
            }
            _ => continue,
        };
    }

    Ok((document, defines))
}

fn expand_group(
    group: &Group,
    environment: &mut Environment,
    depleted: &mut Depleted,
) -> Result<Vec<PreToken>, PreprocessorError> {
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
                tokens.append(&mut expand_text_line(text_line)?);
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
) -> Result<Option<Vec<PreToken>>, PreprocessorError> {
    let condition = expand_region(&if_like.tokens, environment, depleted)?;

    let expression =
        ExprParser::parse(condition.iter(), if_like.source).map_err(PreprocessorError::from)?;

    if expression.is_true() {
        Ok(Some(expand_group(&if_like.group, environment, depleted)?))
    } else {
        Ok(None)
    }
}

fn expand_if_def_like(
    if_def_like: &IfDefLike,
    environment: &mut Environment,
    depleted: &mut Depleted,
) -> Result<Option<Vec<PreToken>>, PreprocessorError> {
    let invert = match if_def_like.kind {
        IfDefKind::Defined => false,
        IfDefKind::NotDefined => true,
    };

    let defined = environment.find_define(&if_def_like.identifier).is_some();

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
) -> Result<Vec<PreToken>, PreprocessorError> {
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
        expand_group(else_group, environment, depleted)
    } else {
        Ok(vec![])
    }
}

fn expand_text_line(text_line: &TextLine) -> Result<Vec<PreToken>, PreprocessorError> {
    Ok(text_line.content.to_vec())
}
