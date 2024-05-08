use super::{
    ast::{
        ControlLine, Define, DefineKind, ElifGroup, Group, IfDefKind, IfDefLike, IfGroup, IfLike,
        IfSection, PreprocessorAst, TextLine,
    },
    token::{PreToken, PreTokenKind},
    PreprocessorError,
};
use crate::c::preprocessor::ast::GroupPart;
use itertools::Itertools;
use std::{
    collections::{HashMap, VecDeque},
    hash::{DefaultHasher, Hash, Hasher},
    time::Duration,
};

#[derive(Clone, Debug, Default)]
pub struct Environment {
    pub defines: HashMap<String, Vec<Define>>,
}

impl Environment {
    pub fn add_define(&mut self, define: &Define) {
        let existing = self.defines.get_mut(&define.name);

        if let Some(existing) = existing {
            for (i, old_define) in existing.iter().enumerate() {
                if define.overwrites(old_define) {
                    existing.remove(i);
                    existing.push(define.clone());
                    return;
                }
            }
        }

        self.defines
            .insert(define.name.clone(), vec![define.clone()]);
    }

    pub fn find_define(&self, name: &str, arity: Option<usize>) -> Option<&Define> {
        for define in self.defines.get(name).into_iter().flatten() {
            if match &define.kind {
                DefineKind::Normal(_) => arity.is_none(),
                DefineKind::Macro(m) => arity.map_or(false, |arity| {
                    arity == m.parameters.len() || (arity > m.parameters.len() && m.is_variadic)
                }),
            } {
                return Some(define);
            }
        }

        None
    }

    pub fn find_defines_of_name(&self, name: &str) -> Option<&Vec<Define>> {
        self.defines.get(name)
    }

    pub fn remove_define(&mut self, name: &str) {
        self.defines.remove(name);
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

fn expand_include(
    files: &[PreToken],
    environment: &mut Environment,
    depleted: &mut Depleted,
) -> Result<Vec<Token>, PreprocessorError> {
    let files = expand_region(files, environment, depleted)?;

    if files.len() != 1 {
        return Err(PreprocessorError::BadInclude);
    }

    // We can choose to satisfy these includes however we want
    match &files.first().unwrap().kind {
        PreTokenKind::HeaderName(header_name) => eprintln!("including <{}>", header_name),
        PreTokenKind::StringLiteral(_encoding, header_name) => {
            eprintln!("including \"{}\"", header_name)
        }
        _ => return Err(PreprocessorError::BadInclude),
    }

    std::thread::sleep(Duration::from_millis(1000));

    Ok(vec![])
}

fn expand_control_line(
    control_line: &ControlLine,
    environment: &mut Environment,
    depleted: &mut Depleted,
) -> Result<Vec<Token>, PreprocessorError> {
    match control_line {
        ControlLine::Include(files) => expand_include(files, environment, depleted),
        ControlLine::Embed(_) => unimplemented!("#embed expansion not implemented yet"),
        ControlLine::Define(define) => {
            environment.add_define(define);
            Ok(vec![])
        }
        ControlLine::Undef(identifier) => {
            environment.remove_define(identifier);
            Ok(vec![])
        }
        ControlLine::Line(_) => unimplemented!("#line expansion not implemented yet"),
        ControlLine::Error(_) => unimplemented!("#error expansion not implemented yet"),
        ControlLine::Warning(_) => unimplemented!("#warning expansion not implemented yet"),
        ControlLine::Pragma(_) => unimplemented!("#pragma expansion not implemented yet"),
    }
}

fn expand_text_line(
    text_line: &TextLine,
    environment: &Environment,
    depleted: &mut Depleted,
) -> Result<Vec<Token>, PreprocessorError> {
    tokenize(&expand_region(&text_line.content, environment, depleted)?)
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
