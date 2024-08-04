use super::error::{ResolveError, ResolveErrorKind};
use crate::{ast, resolved, source_files::Source};
use indexmap::IndexMap;

#[derive(Clone, Debug, Default)]
pub struct TypeSearchCtx<'a> {
    types: IndexMap<String, resolved::TypeKind>,
    type_aliases: IndexMap<String, &'a ast::TypeAlias>,
}

impl<'a> TypeSearchCtx<'a> {
    pub fn new(aliases: IndexMap<String, &'a ast::TypeAlias>) -> Self {
        Self {
            types: Default::default(),
            type_aliases: aliases,
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
        self.type_aliases.get(name).copied()
    }

    pub fn put_type(
        &mut self,
        name: impl ToString,
        value: resolved::TypeKind,
        source: Source,
    ) -> Result<(), ResolveError> {
        if self.types.insert(name.to_string(), value).is_some() {
            return Err(ResolveErrorKind::MultipleDefinitionsOfTypeNamed {
                name: name.to_string(),
            }
            .at(source));
        }

        Ok(())
    }

    pub fn put_type_alias(
        &mut self,
        name: impl ToString,
        value: &'a ast::TypeAlias,
        source: Source,
    ) -> Result<(), ResolveError> {
        if self.type_aliases.insert(name.to_string(), value).is_some() {
            return Err(ResolveErrorKind::MultipleDefinitionsOfTypeNamed {
                name: name.to_string(),
            }
            .at(source));
        }

        Ok(())
    }
}
