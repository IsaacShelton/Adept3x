use super::error::{ResolveError, ResolveErrorKind};
use crate::{
    resolved::{self, VariableStorageKey},
    source_files::{Source, SourceFiles},
};
use std::collections::{HashMap, VecDeque};

#[derive(Clone, Debug)]
pub struct ScopedVariable {
    pub resolved_type: resolved::Type,
    pub key: VariableStorageKey,
}

impl ScopedVariable {
    pub fn new(resolved_type: resolved::Type, key: VariableStorageKey) -> Self {
        Self { resolved_type, key }
    }
}

#[derive(Clone, Debug)]
pub struct VariableSearchCtx<'a> {
    source_files: &'a SourceFiles,
    variables: VecDeque<HashMap<String, ScopedVariable>>,
}

impl<'a> VariableSearchCtx<'a> {
    pub fn new(source_files: &'a SourceFiles) -> Self {
        let mut variables = VecDeque::with_capacity(16);
        variables.push_front(HashMap::new());

        Self {
            source_files,
            variables,
        }
    }

    pub fn find_variable_or_error(
        &self,
        name: &str,
        source: Source,
    ) -> Result<&ScopedVariable, ResolveError> {
        match self.find_variable(name) {
            Some(variable) => Ok(variable),
            None => Err(ResolveErrorKind::UndeclaredVariable {
                name: name.to_string(),
            }
            .at(source)),
        }
    }

    pub fn find_variable(&self, name: &str) -> Option<&ScopedVariable> {
        for variables in self.variables.iter() {
            if let Some(scoped_variable) = variables.get(name) {
                return Some(scoped_variable);
            }
        }

        None
    }

    pub fn put(
        &mut self,
        name: impl ToString,
        resolved_type: resolved::Type,
        key: VariableStorageKey,
    ) {
        self.variables
            .front_mut()
            .expect("at least one scope")
            .insert(name.to_string(), ScopedVariable::new(resolved_type, key));
    }

    pub fn begin_scope(&mut self) {
        self.variables.push_front(Default::default());
    }

    pub fn end_scope(&mut self) {
        self.variables.pop_front().expect("scope to close");
    }
}
