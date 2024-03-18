use super::error::{ResolveError, ResolveErrorKind};
use crate::{ast::Source, resolved, source_file_cache::SourceFileCache};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct TypeSearchContext<'a> {
    source_file_cache: &'a SourceFileCache,
    types: HashMap<String, resolved::Type>,
}

impl<'a> TypeSearchContext<'a> {
    pub fn new(source_file_cache: &'a SourceFileCache) -> Self {
        Self {
            source_file_cache,
            types: Default::default(),
        }
    }

    pub fn find_type_or_error(
        &self,
        name: &str,
        source: Source,
    ) -> Result<&resolved::Type, ResolveError> {
        match self.find_type(name) {
            Some(info) => Ok(info),
            None => Err(ResolveError {
                filename: Some(
                    self.source_file_cache
                        .get(source.key)
                        .filename()
                        .to_string(),
                ),
                location: Some(source.location),
                kind: ResolveErrorKind::UndeclaredType {
                    name: name.to_string(),
                },
            }),
        }
    }

    pub fn find_type(&self, name: &str) -> Option<&resolved::Type> {
        if let Some(resolved_type) = self.types.get(name) {
            return Some(resolved_type);
        }
        None
    }

    pub fn put(&mut self, name: impl ToString, resolved_type: resolved::Type) {
        self.types.insert(name.to_string(), resolved_type);
    }
}
