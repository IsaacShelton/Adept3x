use super::error::{ResolveError, ResolveErrorKind};
use crate::{ast::Source, resolved, source_file_cache::SourceFileCache};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct TypeSearchCtx<'a> {
    source_file_cache: &'a SourceFileCache,
    types: HashMap<String, resolved::TypeKind>,
}

impl<'a> TypeSearchCtx<'a> {
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
    ) -> Result<&resolved::TypeKind, ResolveError> {
        match self.find_type(name) {
            Some(info) => Ok(info),
            None => Err(ResolveErrorKind::UndeclaredType {
                name: name.to_string(),
            }
            .at(source)),
        }
    }

    pub fn find_type(&self, name: &str) -> Option<&resolved::TypeKind> {
        self.types.get(name)
    }

    pub fn put(
        &mut self,
        name: impl ToString,
        value: resolved::TypeKind,
        source: Source,
    ) -> Result<(), ResolveError> {
        if self.types.insert(name.to_string(), value).is_none() {
            Ok(())
        } else {
            Err(ResolveErrorKind::MultipleDefinitionsOfTypeNamed {
                name: name.to_string(),
            }
            .at(source))
        }
    }
}
