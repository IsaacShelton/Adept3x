use super::error::{ResolveError, ResolveErrorKind};
use crate::{
    name::{Name, ResolvedName},
    resolved,
    source_files::Source,
    workspace::fs::FsNodeId,
};
use std::borrow::Cow;

pub fn find_type<'a>(
    resolved_ast: &'a resolved::Ast,
    module_node_id: FsNodeId,
    name: &Name,
) -> Result<Cow<'a, resolved::TypeKind>, FindTypeError> {
    let _source_files = resolved_ast.source_files;
    let _settings = resolved_ast
        .workspace
        .get_settings_for_module(module_node_id);
    let _all_types = &resolved_ast.all_types;

    if let Some(_name) = name.as_plain_str() {
        todo!("TypeSearchCtx find_type for local type");
    }

    todo!("TypeSearchCtx find_type");

    /*
    let resolved_name = ResolvedName::new(self.fs_node_id, name);

    if let Some(mapping) = self.types.get(&resolved_name) {
        return self.resolve_mapping(&resolved_name, mapping, used_aliases_stack);
    }

    if name.namespace.is_empty() {
        let mut matches = self
            .settings
            .imported_namespaces
            .iter()
            .filter_map(|namespace| {
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
    */

    Err(FindTypeError::NotDefined)
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
