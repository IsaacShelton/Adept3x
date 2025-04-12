use super::{ResolveTypeCtx, ResolveTypeOptions, find_error::FindTypeError};
use crate::error::ResolveError;
use asg::{TypeParam, TypeParamError};
use ast::{Name, TypeArg};
use source_files::Source;
use std::borrow::Cow;

impl<'a> ResolveTypeCtx<'a> {
    pub fn find(
        &self,
        name: &Name,
        type_args: &[TypeArg],
        source: Source,
    ) -> Result<Cow<'a, asg::TypeKind>, FindTypeError> {
        let settings = &self.asg.workspace.settings[self
            .asg
            .workspace
            .files
            .get(self.file_fs_node_id)
            .unwrap()
            .settings
            .expect("valid settings id")];

        let decl = name
            .as_plain_str()
            .and_then(|name| {
                self.types_in_modules
                    .get(&self.module_fs_node_id)
                    .and_then(|types_in_module| types_in_module.get(name))
                    .filter(|ty_decl| {
                        !ty_decl.privacy.is_private()
                            || self.file_fs_node_id == ty_decl.file_fs_node_id
                    })
            })
            .filter(|local| local.num_parameters(self.asg) == type_args.len())
            .map(Ok)
            .unwrap_or_else(|| {
                if name.namespace.is_empty() {
                    return Err(FindTypeError::NotDefined);
                }

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
                    .filter(|decl| decl.num_parameters(self.asg) == type_args.len());

                if let Some(found) = matches.next() {
                    if matches.next().is_some() {
                        Err(FindTypeError::Ambiguous)
                    } else {
                        Ok(found)
                    }
                } else {
                    Err(FindTypeError::NotDefined)
                }
            })?;

        let mut type_args = type_args.into_iter().enumerate();

        let filled = decl
            .kind
            .map_type_params(|_hint| {
                let Some((_i, value)) = type_args.next() else {
                    return Err(FindTypeError::TypeArgsLengthMismatch);
                };

                match value {
                    TypeArg::Type(ty) => self
                        .resolve(ty, ResolveTypeOptions::Unalias)
                        .map(Cow::Owned)
                        .map(TypeParam::Type)
                        .map_err(FindTypeError::ResolveError),
                    TypeArg::Expr(expr) => {
                        let ast::ExprKind::Integer(ast::Integer::Generic(value)) = &expr.kind
                        else {
                            return Err(FindTypeError::ResolveError(ResolveError::other(
                                "Expressions are not supported as type arguments yet",
                                source,
                            )));
                        };

                        u64::try_from(value).map(TypeParam::Size).map_err(|_| {
                            FindTypeError::ResolveError(ResolveError::other(
                                "Size is too large",
                                source,
                            ))
                        })
                    }
                }
            })
            .map_err(|err| match err {
                TypeParamError::MappingError(e) => e,
                TypeParamError::ExpectedType { index } => {
                    FindTypeError::ResolveError(ResolveError::other(
                        format!("Expected type for type argument {}", index + 1),
                        source,
                    ))
                }
                TypeParamError::ExpectedSize { index } => {
                    FindTypeError::ResolveError(ResolveError::other(
                        format!("Expected size for type argument {}", index + 1),
                        source,
                    ))
                }
                TypeParamError::ExpectedSizeValue { index, value } => {
                    FindTypeError::ResolveError(ResolveError::other(
                        format!("Expected size of {} of type argument {}", value, index + 1),
                        source,
                    ))
                }
            });

        if type_args.next().is_some() {
            return Err(FindTypeError::TypeArgsLengthMismatch);
        };

        filled
    }
}
