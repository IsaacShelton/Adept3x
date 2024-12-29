use super::{find_error::FindTypeError, ResolveTypeCtx};
use crate::{
    ast::CompileTimeArgument,
    name::Name,
    resolve::error::ResolveErrorKind,
    asg::{self},
};
use itertools::Itertools;
use std::borrow::Cow;

impl<'a> ResolveTypeCtx<'a> {
    pub fn find(
        &self,
        name: &Name,
        arguments: &[CompileTimeArgument],
    ) -> Result<Cow<'a, asg::TypeKind>, FindTypeError> {
        let settings = &self.asg.workspace.settings[self
            .asg
            .workspace
            .files
            .get(&self.file_fs_node_id)
            .unwrap()
            .settings
            .expect("valid settings id")
            .0];

        if let Some(decl) = name
            .as_plain_str()
            .and_then(|name| {
                self.types_in_modules
                    .get(&self.module_fs_node_id)
                    .and_then(|types_in_module| types_in_module.get(name))
            })
            // NOTE: This will need to be map instead at some point
            .filter(|decl| decl.num_parameters(self.asg) == arguments.len())
        {
            if let asg::TypeKind::Structure(human_name, struct_ref, _) = &decl.kind {
                let arguments = arguments
                    .iter()
                    .flat_map(|arg| match arg {
                        CompileTimeArgument::Type(ty) => self.resolve(ty),
                        CompileTimeArgument::Expr(expr) => Err(ResolveErrorKind::Other {
                            message:
                                "Expressions cannot be used as type parameters to structures yet"
                                    .into(),
                        }
                        .at(expr.source)),
                    })
                    .collect_vec();

                return Ok(Cow::Owned(asg::TypeKind::Structure(
                    human_name.clone(),
                    *struct_ref,
                    arguments,
                )));
            }

            if let asg::TypeKind::Trait(human_name, trait_ref, _) = &decl.kind {
                let arguments = arguments
                    .iter()
                    .flat_map(|arg| match arg {
                        CompileTimeArgument::Type(ty) => self.resolve(ty),
                        CompileTimeArgument::Expr(expr) => Err(ResolveErrorKind::Other {
                            message: "Expressions cannot be used as type parameters to traits yet"
                                .into(),
                        }
                        .at(expr.source)),
                    })
                    .collect_vec();

                return Ok(Cow::Owned(asg::TypeKind::Trait(
                    human_name.clone(),
                    *trait_ref,
                    arguments,
                )));
            }

            return Ok(Cow::Borrowed(&decl.kind));
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
                .filter(|decl| decl.privacy.is_public())
                // NOTE: This will need to be flat_map instead at some point
                .filter(|decl| decl.num_parameters(self.asg) == arguments.len());

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
