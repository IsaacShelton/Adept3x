use super::{
    error::{ResolveError, ResolveErrorKind},
    resolve_type,
};
use crate::{
    ast,
    name::{Name, ResolvedName},
    resolved,
    source_files::{Source, SourceFiles},
    workspace::fs::FsNodeId,
};
use indexmap::IndexMap;
use std::{borrow::Cow, collections::HashSet};

#[derive(Clone, Debug)]
pub struct TypeSearchCtx<'a> {
    types: IndexMap<ResolvedName, TypeMapping<'a>>,
    imported_namespaces: Vec<Box<str>>,
    source_files: &'a SourceFiles,
    fs_node_id: FsNodeId,
}

#[derive(Clone, Debug)]
pub enum TypeMapping<'a> {
    Normal(resolved::TypeKind),
    Alias(&'a ast::TypeAlias),
}

#[derive(Clone, Debug)]
pub enum FindTypeError {
    NotDefined,
    Ambiguous,
    RecursiveAlias(ResolvedName),
    ResolveError(ResolveError),
}

impl FindTypeError {
    pub fn into_resolve_error(self: FindTypeError, name: &Name, source: Source) -> ResolveError {
        let name = name.to_string();

        match self {
            FindTypeError::NotDefined => ResolveErrorKind::UndeclaredType {
                name: name.to_string(),
            }
            .at(source),
            FindTypeError::Ambiguous => ResolveErrorKind::AmbiguousType {
                name: name.to_string(),
            }
            .at(source),
            FindTypeError::RecursiveAlias(_) => ResolveErrorKind::RecursiveTypeAlias {
                name: name.to_string(),
            }
            .at(source),
            FindTypeError::ResolveError(err) => err,
        }
    }
}

impl<'a> TypeSearchCtx<'a> {
    pub fn new(
        imported_namespaces: Vec<Box<str>>,
        source_files: &'a SourceFiles,
        fs_node_id: FsNodeId,
    ) -> Self {
        Self {
            types: Default::default(),
            imported_namespaces,
            source_files,
            fs_node_id,
        }
    }

    pub fn find_type(
        &'a self,
        name: &Name,
        used_aliases_stack: &mut HashSet<ResolvedName>,
    ) -> Result<Cow<'a, resolved::TypeKind>, FindTypeError> {
        let resolved_name = ResolvedName::new(self.fs_node_id, name);

        if let Some(mapping) = self.types.get(&resolved_name) {
            return self.resolve_mapping(&resolved_name, mapping, used_aliases_stack);
        }

        if name.namespace.is_empty() {
            let mut matches = self.imported_namespaces.iter().filter_map(|namespace| {
                let resolved_name = ResolvedName::new(
                    self.fs_node_id,
                    &Name::new(Some(namespace.clone()), name.basename.clone()),
                );
                self.types.get(&resolved_name)
            });

            if let Some(found) = matches.next() {
                if matches.next().is_some() {
                    return Err(FindTypeError::Ambiguous);
                } else {
                    return self.resolve_mapping(&resolved_name, found, used_aliases_stack);
                }
            }
        }

        Err(FindTypeError::NotDefined)
    }

    pub fn resolve_mapping(
        &self,
        resolved_name: &ResolvedName,
        mapping: &'a TypeMapping,
        used_aliases_stack: &mut HashSet<ResolvedName>,
    ) -> Result<Cow<'a, resolved::TypeKind>, FindTypeError> {
        match mapping {
            TypeMapping::Normal(kind) => Ok(Cow::Borrowed(kind)),
            TypeMapping::Alias(alias) => {
                if used_aliases_stack.insert(resolved_name.clone()) {
                    let inner = resolve_type(self, &alias.value, used_aliases_stack)
                        .map_err(FindTypeError::ResolveError)?;
                    used_aliases_stack.remove(&resolved_name);
                    Ok(Cow::Owned(inner.kind.clone()))
                } else {
                    Err(FindTypeError::RecursiveAlias(resolved_name.clone()))
                }
            }
        }
    }

    pub fn put_type(
        &mut self,
        name: &Name,
        value: resolved::TypeKind,
        source: Source,
    ) -> Result<(), ResolveError> {
        let resolved_name = ResolvedName::new(self.fs_node_id, name);

        if self
            .types
            .insert(resolved_name, TypeMapping::Normal(value))
            .is_some()
        {
            return Err(ResolveErrorKind::MultipleDefinitionsOfTypeNamed {
                name: name.to_string(),
            }
            .at(source));
        }

        Ok(())
    }

    pub fn override_type(&mut self, name: &Name, value: resolved::TypeKind) {
        let resolved_name = ResolvedName::new(self.fs_node_id, name);
        self.types.insert(resolved_name, TypeMapping::Normal(value));
    }

    pub fn put_type_alias(
        &mut self,
        name: &Name,
        value: &'a ast::TypeAlias,
        source: Source,
    ) -> Result<(), ResolveError> {
        let resolved_name = ResolvedName::new(self.fs_node_id, &name);

        if self
            .types
            .insert(resolved_name, TypeMapping::Alias(value))
            .is_some()
        {
            return Err(ResolveErrorKind::MultipleDefinitionsOfTypeNamed {
                name: name.to_string(),
            }
            .at(source));
        }

        Ok(())
    }
}
