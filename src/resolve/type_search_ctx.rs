use super::error::{ResolveError, ResolveErrorKind};
use crate::{
    ast, resolved,
    source_files::{Source, SourceFiles},
};
use indexmap::IndexMap;

#[derive(Clone, Debug)]
pub struct TypeSearchCtx<'a> {
    source_files: &'a SourceFiles,
    types: IndexMap<String, resolved::TypeKind>,
    aliases: IndexMap<String, &'a ast::TypeAlias>,
}

impl<'a> TypeSearchCtx<'a> {
    pub fn new(
        source_files: &'a SourceFiles,
        aliases: IndexMap<String, &'a ast::TypeAlias>,
    ) -> Self {
        Self {
            source_files,
            types: Default::default(),
            aliases,
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

    pub fn find_alias(&self, name: &str) -> Option<&ast::TypeAlias> {
        self.aliases.get(name).copied()
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
