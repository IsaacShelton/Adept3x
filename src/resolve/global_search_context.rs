use super::error::{ResolveError, ResolveErrorKind};
use crate::{
    ast::Source,
    resolved::{self, GlobalRef},
    source_file_cache::SourceFileCache,
};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct GlobalSearchContext<'a> {
    source_file_cache: &'a SourceFileCache,
    globals: HashMap<String, (resolved::Type, GlobalRef)>,
}

impl<'a> GlobalSearchContext<'a> {
    pub fn new(source_file_cache: &'a SourceFileCache) -> Self {
        Self {
            source_file_cache,
            globals: Default::default(),
        }
    }

    pub fn find_global_or_error(
        &self,
        name: &str,
        source: Source,
    ) -> Result<(&resolved::Type, &GlobalRef), ResolveError> {
        match self.find_global(name) {
            Some(global) => Ok(global),
            None => Err(ResolveError::new(
                self.source_file_cache,
                source,
                ResolveErrorKind::UndeclaredVariable {
                    name: name.to_string(),
                },
            )),
        }
    }

    pub fn find_global(&self, name: &str) -> Option<(&resolved::Type, &GlobalRef)> {
        if let Some((resolved_type, key)) = self.globals.get(name) {
            return Some((resolved_type, key));
        }
        None
    }

    pub fn put(
        &mut self,
        name: impl ToString,
        resolved_type: resolved::Type,
        reference: GlobalRef,
    ) {
        self.globals
            .insert(name.to_string(), (resolved_type, reference));
    }
}
