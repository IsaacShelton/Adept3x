use super::error::{ErrorInfo, ResolveError};
use crate::{
    error::CompilerError,
    resolved::{self, VariableStorageKey},
};
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct VariableSearchContext<'a> {
    variables: HashMap<String, (resolved::Type, VariableStorageKey)>,
    parent: Option<&'a VariableSearchContext<'a>>,
}

impl<'a> VariableSearchContext<'a> {
    pub fn find_variable_or_error(
        &self,
        name: &str,
    ) -> Result<(&resolved::Type, &VariableStorageKey), ResolveError> {
        match self.find_variable(name) {
            Some(function) => Ok(function),
            None => Err(ResolveError {
                filename: todo!(),
                location: todo!(),
                info: ErrorInfo::UndeclaredVariable {
                    name: name.to_string(),
                },
            }),
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
