use super::{find_error::FindTypeError, ResolveTypeCtx};
use crate::{name::Name, resolved};
use std::borrow::Cow;

impl<'a> ResolveTypeCtx<'a> {
    pub fn find(&self, name: &Name) -> Result<Cow<'a, resolved::TypeKind>, FindTypeError> {
        let settings = &self.resolved_ast.workspace.settings[self
            .resolved_ast
            .workspace
            .files
            .get(&self.file_fs_node_id)
            .unwrap()
            .settings
            .expect("valid settings id")
            .0];

        if let Some(name) = name.as_plain_str() {
            if let Some(types_in_module) = self.types_in_modules.get(&self.module_fs_node_id) {
                if let Some(decl) = types_in_module.get(name) {
                    return Ok(Cow::Borrowed(&decl.kind));
                }
            }
        }

        if !name.namespace.is_empty() {
            let Name {
                namespace,
                basename,
                ..
            } = name;

            let mut matches = settings
                .namespace_to_dependency
                .get(namespace.as_ref())
                .into_iter()
                .flatten()
                .flat_map(|dep| settings.dependency_to_module.get(dep))
                .flat_map(|fs_node_id| self.types_in_modules.get(fs_node_id))
                .flat_map(|decls| decls.get(basename.as_ref()))
                .filter(|decl| decl.privacy.is_public());

            if let Some(found) = matches.next() {
                if matches.next().is_some() {
                    return Err(FindTypeError::Ambiguous);
                } else {
                    return Ok(Cow::Borrowed(&found.kind));
                }
            }
        }

        Err(FindTypeError::NotDefined)
    }
}
