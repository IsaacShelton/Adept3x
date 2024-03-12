use super::error::{ResolveError, ResolveErrorKind};
use crate::{
    ast::Source,
    error::CompilerError,
    resolved::{self, GlobalRef, VariableStorageKey},
    source_file_cache::{self, SourceFileCache},
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
            None => Err(ResolveError {
                filename: Some(
                    self.source_file_cache
                        .get(source.key)
                        .filename()
                        .to_string(),
                ),
                location: Some(source.location),
                kind: ResolveErrorKind::UndeclaredVariable {
                    name: name.to_string(),
                },
            }),
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
