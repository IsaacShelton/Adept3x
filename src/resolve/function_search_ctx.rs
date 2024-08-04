use super::error::{ResolveError, ResolveErrorKind};
use crate::{resolved, source_files::Source};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct FunctionSearchCtx {
    pub available: HashMap<String, Vec<resolved::FunctionRef>>,
}

impl FunctionSearchCtx {
    pub fn new() -> Self {
        Self {
            available: Default::default(),
        }
    }

    pub fn find_function_or_error(
        &self,
        name: &str,
        source: Source,
    ) -> Result<resolved::FunctionRef, ResolveError> {
        match self.find_function(name) {
            Some(function) => Ok(function),
            None => Err(ResolveErrorKind::FailedToFindFunction {
                name: name.to_string(),
            }
            .at(source)),
        }
    }

    pub fn find_function(&self, name: &str) -> Option<resolved::FunctionRef> {
        self.available
            .get(name)
            .and_then(|list| list.first())
            .copied()
    }
}
