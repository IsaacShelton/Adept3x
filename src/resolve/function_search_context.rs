use super::error::{ResolveError, ResolveErrorKind};
use crate::{ast::Source, resolved, source_file_cache::SourceFileCache};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct FunctionSearchContext<'a> {
    pub available: HashMap<String, Vec<resolved::FunctionRef>>,

    source_file_cache: &'a SourceFileCache,
}

impl<'a> FunctionSearchContext<'a> {
    pub fn new(source_file_cache: &'a SourceFileCache) -> Self {
        Self {
            available: Default::default(),
            source_file_cache,
        }
    }

    pub fn find_function_or_error(
        &self,
        name: &str,
        source: Source,
    ) -> Result<resolved::FunctionRef, ResolveError> {
        match self.find_function(name) {
            Some(function) => Ok(function),
            None => Err(ResolveError::new(
                self.source_file_cache,
                source,
                ResolveErrorKind::FailedToFindFunction {
                    name: name.to_string(),
                },
            )),
        }
    }

    pub fn find_function(&self, name: &str) -> Option<resolved::FunctionRef> {
        self.available
            .get(name)
            .and_then(|list| list.get(0))
            .copied()
    }
}
