use super::error::{ResolveError, ResolveErrorKind};
use crate::{ast, name::ResolvedName, resolved, source_files::Source};
use indexmap::IndexMap;

#[derive(Clone, Debug, Default)]
pub struct TypeSearchCtx<'a> {
    types: IndexMap<ResolvedName, resolved::TypeKind>,
    type_aliases: IndexMap<ResolvedName, &'a ast::TypeAlias>,
}

impl<'a> TypeSearchCtx<'a> {
    pub fn new(aliases: IndexMap<ResolvedName, &'a ast::TypeAlias>) -> Self {
        Self {
            types: Default::default(),
            type_aliases: aliases,
        }
    }

    pub fn find_type(&self, name: &ResolvedName) -> Option<&resolved::TypeKind> {
        self.types.get(name)
    }

    pub fn find_alias(&self, name: &ResolvedName) -> Option<&ast::TypeAlias> {
        self.type_aliases.get(name).copied()
    }

    pub fn put_type(
        &mut self,
        name: impl ToString,
        value: resolved::TypeKind,
        source: Source,
    ) -> Result<(), ResolveError> {
        eprintln!("warning: TypeSearchCtx::put_type always puts at root");
        let resolved_name = ResolvedName::Project(name.to_string().into_boxed_str());

        if self.types.insert(resolved_name, value).is_some() {
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
        eprintln!("warning: TypeSearchCtx::put_type_alias always puts at root");
        let resolved_name = ResolvedName::Project(name.to_string().into_boxed_str());

        if self.type_aliases.insert(resolved_name, value).is_some() {
            return Err(ResolveErrorKind::MultipleDefinitionsOfTypeNamed {
                name: name.to_string(),
            }
            .at(source));
        }

        Ok(())
    }
}
