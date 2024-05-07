use super::{
    ast::{ControlLine, Define, DefineKind, PreprocessorAst, TextLine},
    token::{PreToken, PreTokenKind},
    PreprocessorError,
};
use crate::c::preprocessor::ast::GroupPart;
use itertools::Itertools;
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct Environment {
    pub defines: HashMap<String, Vec<Define>>,
}

impl Environment {
    pub fn add_define(&mut self, define: &Define) {
        let existing = self.defines.get_mut(&define.name);

        if let Some(existing) = existing {
            for (i, def) in existing.iter().enumerate() {
                if define.overwrites(def) {
                    existing.remove(i);
                    existing.push(define.clone());
                    return;
                }
            }

            self.defines
                .insert(define.name.clone(), vec![define.clone()]);
        } else {
            self.defines
                .insert(define.name.clone(), vec![define.clone()]);
        }
    }
}

#[derive(Clone, Debug)]
pub enum Token {
    PreToken(PreToken),
}

pub fn expand_ast(
    ast: &PreprocessorAst,
    environment: Environment,
) -> Result<Vec<Token>, PreprocessorError> {
    let mut environment = environment;
    let mut tokens = Vec::with_capacity(1024);

    for part in ast.group.parts.iter() {
        match part {
            GroupPart::IfSection(_) => todo!(),
            GroupPart::ControlLine(control_line) => {
                expand_control_line(control_line, &mut environment)?
            }
            GroupPart::TextLine(text_line) => {
                tokens.append(&mut expand_text_line(text_line, &environment)?)
            }
            GroupPart::HashNonDirective => todo!(),
        }
    }

    Ok(tokens)
}

fn expand_control_line(
    control_line: &ControlLine,
    environment: &mut Environment,
) -> Result<(), PreprocessorError> {
    match control_line {
        ControlLine::Include(_) => todo!(),
        ControlLine::Embed(_) => todo!(),
        ControlLine::Define(define) => {
            environment.add_define(define);
            Ok(())
        }
        ControlLine::Undef(_) => todo!(),
        ControlLine::Line(_) => todo!(),
        ControlLine::Error(_) => todo!(),
        ControlLine::Warning(_) => todo!(),
        ControlLine::Pragma(_) => todo!(),
    }
}

fn expand_text_line(
    text_line: &TextLine,
    environment: &Environment,
) -> Result<Vec<Token>, PreprocessorError> {
    tokenize(&expand_region(&text_line.content, environment)?)
}

fn expand_region(
    original_tokens: &[PreToken],
    environment: &Environment,
) -> Result<Vec<PreToken>, PreprocessorError> {
    let mut expanded = Vec::<PreToken>::with_capacity(original_tokens.len() + 16);
    expanded.extend(original_tokens.iter().cloned());

    let mut i = 0;

    while i < expanded.len() {
        match &expanded[i].kind {
            PreTokenKind::Identifier(definition_name) => {
                // TODO: Incorporate list of values already expanded for regions,
                // and determine if macros of the same name but different definitions should be
                // able to run, etc.

                // TODO: Definitions cannot be used in their own expansion, and should be left
                // untouched in that case according to the standard. This requirement still
                // needs to be implementated.

                let define = if let Some(definitions) = environment.defines.get(definition_name) {
                    definitions.first()
                } else {
                    None
                };

                if let Some(define) = define {
                    match &define.kind {
                        DefineKind::Normal(replacement) => {
                            expanded.splice(i..=i, replacement.iter().cloned());
                        }
                        DefineKind::Macro(_) => todo!(),
                    }

                    i = 0;
                    continue;
                }
            }
            PreTokenKind::HeaderName(_)
            | PreTokenKind::Number(_)
            | PreTokenKind::CharacterConstant(_, _)
            | PreTokenKind::StringLiteral(_, _)
            | PreTokenKind::Punctuator(_)
            | PreTokenKind::UniversalCharacterName(_)
            | PreTokenKind::Other(_) => (),
        }

        i += 1;
    }

    Ok(expanded)
}

fn tokenize(tokens: &[PreToken]) -> Result<Vec<Token>, PreprocessorError> {
    Ok(tokens
        .iter()
        .map(|token| Token::PreToken(token.clone()))
        .collect_vec())
}
