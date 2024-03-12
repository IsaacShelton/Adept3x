use crate::{
    ast::Source, error::CompilerError, resolved, source_file_cache::{self, SourceFileCache}
};
use std::collections::HashMap;

use super::error::{ResolveError, ResolveErrorKind};

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
            None => Err(ResolveError {
                filename: Some(self.source_file_cache.get(source.key).filename().to_string()),
                location: Some(source.location),
                kind: ResolveErrorKind::FailedToFindFunction {
                    name: name.to_string(),
                },
            }),
        }
    }

    pub fn find_function(&self, name: &str) -> Option<resolved::FunctionRef> {
        self.available
            .get(name)
            .and_then(|list| list.get(0))
            .copied()
    }
}
