use super::error::{ResolveError, ResolveErrorKind};
use crate::{
    ast::Source,
    resolved::{self, VariableStorageKey},
    source_file_cache::SourceFileCache,
};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct VariableSearchContext<'a> {
    source_file_cache: &'a SourceFileCache,
    variables: HashMap<String, (resolved::Type, VariableStorageKey)>,
    parent: Option<&'a VariableSearchContext<'a>>,
}

impl<'a> VariableSearchContext<'a> {
    pub fn new(source_file_cache: &'a SourceFileCache) -> Self {
        Self {
            source_file_cache,
            variables: Default::default(),
            parent: None,
        }
    }

    pub fn find_variable_or_error(
        &self,
        name: &str,
        source: Source,
    ) -> Result<(&resolved::Type, &VariableStorageKey), ResolveError> {
        match self.find_variable(name) {
            Some(variable) => Ok(variable),
            None => Err(ResolveError::new(
                self.source_file_cache,
                source,
                ResolveErrorKind::UndeclaredVariable {
                    name: name.to_string(),
                },
            )),
        }
    }

    pub fn find_variable(&self, name: &str) -> Option<(&resolved::Type, &VariableStorageKey)> {
        if let Some((resolved_type, key)) = self.variables.get(name) {
            return Some((resolved_type, key));
        }

        self.parent
            .as_ref()
            .and_then(|parent| parent.find_variable(name))
    }

    pub fn put(
        &mut self,
        name: impl ToString,
        resolved_type: resolved::Type,
        key: VariableStorageKey,
    ) {
        self.variables
            .insert(name.to_string(), (resolved_type, key));
    }
}
