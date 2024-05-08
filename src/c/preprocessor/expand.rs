use super::{
    ast::{ControlLine, Define, DefineKind, PreprocessorAst, TextLine},
    token::{PreToken, PreTokenKind},
    PreprocessorError,
};
use crate::c::preprocessor::ast::GroupPart;
use itertools::Itertools;
use std::{
    collections::{HashMap, VecDeque},
    hash::{DefaultHasher, Hash, Hasher},
};

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

    pub fn find_define(&self, name: &str, arity: Option<usize>) -> Option<&Define> {
        if let Some(defines) = self.defines.get(name) {
            for define in defines.iter() {
                let matches = match &define.kind {
                    DefineKind::Normal(_) => arity.is_none(),
                    DefineKind::Macro(macro_definition) => {
                        if let Some(arity) = arity {
                            let length = macro_definition.parameters.len();
                            length == arity || (length > arity && macro_definition.is_variadic)
                        } else {
                            false
                        }
                    }
                };

                if matches {
                    return Some(define);
                }
            }

            None
        } else {
            None
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
                tokens.append(&mut expand_control_line(control_line, &mut environment)?);
            }
            GroupPart::TextLine(text_line) => {
                tokens.append(&mut expand_text_line(text_line, &environment)?);
            }
            GroupPart::HashNonDirective => (), // Ignored during expansion
        }
    }

    Ok(tokens)
}

fn expand_control_line(
    control_line: &ControlLine,
    environment: &mut Environment,
) -> Result<Vec<Token>, PreprocessorError> {
    match control_line {
        ControlLine::Include(_) => unimplemented!("#include expansion not implemented yet"),
        ControlLine::Embed(_) => unimplemented!("#embed expansion not implemented yet"),
        ControlLine::Define(define) => {
            environment.add_define(define);
            Ok(vec![])
        }
        ControlLine::Undef(_) => unimplemented!("#undef expansion not implemented yet"),
        ControlLine::Line(_) => unimplemented!("#line expansion not implemented yet"),
        ControlLine::Error(_) => unimplemented!("#error expansion not implemented yet"),
        ControlLine::Warning(_) => unimplemented!("#warning expansion not implemented yet"),
        ControlLine::Pragma(_) => unimplemented!("#pragma expansion not implemented yet"),
    }
}

fn expand_text_line(
    text_line: &TextLine,
    environment: &Environment,
) -> Result<Vec<Token>, PreprocessorError> {
    let mut depleted = Depleted::new();
    tokenize(&expand_region(
        &text_line.content,
        environment,
        &mut depleted,
    )?)
}

struct Depleted {
    pub hashes: VecDeque<u64>,
}

impl Depleted {
    pub fn new() -> Self {
        Self {
            hashes: Default::default(),
        }
    }

    pub fn push(&mut self, define: &Define) {
        self.hashes.push_back(Self::hash_define(define));
    }

    pub fn pop(&mut self) {
        self.hashes.pop_back();
    }

    pub fn contains(&self, define: &Define) -> bool {
        let hash = Self::hash_define(define);

        for item in self.hashes.iter().rev() {
            if *item == hash {
                return true;
            }
        }

        false
    }

    fn hash_define(define: &Define) -> u64 {
        let mut hasher = DefaultHasher::new();
        define.hash(&mut hasher);
        hasher.finish()
    }
}

fn expand_region(
    tokens: &[PreToken],
    environment: &Environment,
    depleted: &mut Depleted,
) -> Result<Vec<PreToken>, PreprocessorError> {
    let mut expanded = Vec::with_capacity(tokens.len() + 16);

    for token in tokens.iter() {
        expanded.append(&mut expand_token(token, environment, depleted)?);
    }

    Ok(expanded)
}

fn expand_token(
    token: &PreToken,
    environment: &Environment,
    depleted: &mut Depleted,
) -> Result<Vec<PreToken>, PreprocessorError> {
    match &token.kind {
        PreTokenKind::Identifier(name) => {
            if let Some(define) = environment.find_define(name, None) {
                if !depleted.contains(define) {
                    depleted.push(define);
                    let replacement = match &define.kind {
                        DefineKind::Normal(replacement) => replacement,
                        DefineKind::Macro(_) => unimplemented!("expanding macro define"),
                    };
                    let expanded = expand_region(&replacement, environment, depleted)?;
                    depleted.pop();
                    return Ok(expanded);
                }
            }

            Ok(vec![token.clone()])
        }
        PreTokenKind::HeaderName(_)
        | PreTokenKind::Number(_)
        | PreTokenKind::CharacterConstant(_, _)
        | PreTokenKind::StringLiteral(_, _)
        | PreTokenKind::Punctuator(_)
        | PreTokenKind::UniversalCharacterName(_)
        | PreTokenKind::Other(_) => Ok(vec![token.clone()]),
    }
}

fn tokenize(tokens: &[PreToken]) -> Result<Vec<Token>, PreprocessorError> {
    Ok(tokens
        .iter()
        .map(|token| Token::PreToken(token.clone()))
        .collect_vec())
}
