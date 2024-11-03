use super::{
    conform::{
        conform_expr, to_default::conform_expr_to_default, warn_type_alias_depth_exceeded,
        ConformMode, Validate,
    },
    expr::{PreferredType, ResolveExprCtx},
};
use crate::{
    ir::FunctionRef,
    name::{Name, ResolvedName},
    resolved::{self, TypeKind, TypedExpr},
    source_files::Source,
    workspace::fs::FsNodeId,
};
use itertools::Itertools;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct FunctionHaystack {
    pub available: HashMap<ResolvedName, Vec<resolved::FunctionRef>>,
    pub imported_namespaces: Vec<Box<str>>,
    pub fs_node_id: FsNodeId,
}

#[derive(Clone, Debug)]
pub enum FindFunctionError {
    NotDefined,
    Ambiguous,
}

impl FunctionHaystack {
    pub fn new(imported_namespaces: Vec<Box<str>>, fs_node_id: FsNodeId) -> Self {
        Self {
            available: Default::default(),
            imported_namespaces,
            fs_node_id,
        }
    }

    pub fn find(
        &self,
        ctx: &ResolveExprCtx,
        name: &Name,
        arguments: &[TypedExpr],
        source: Source,
    ) -> Result<FunctionRef, FindFunctionError> {
        let resolved_name = ResolvedName::new(self.fs_node_id, name);

        let mut local_matches = self
            .available
            .get(&resolved_name)
            .into_iter()
            .flatten()
            .filter(|f| Self::fits(ctx, **f, arguments, source));

        if let Some(found) = local_matches.next() {
            return if local_matches.next().is_some() {
                Err(FindFunctionError::Ambiguous)
            } else {
                Ok(*found)
            };
        }

        let mut remote_matches = (!name.namespace.is_empty())
            .then(|| {
                ctx.settings
                    .namespace_to_dependency
                    .get(name.namespace.as_ref())
            })
            .flatten()
            .into_iter()
            .flatten()
            .flat_map(|dependency| {
                ctx.settings
                    .dependency_to_module
                    .get(dependency)
                    .and_then(|module_fs_node_id| ctx.public_functions.get(module_fs_node_id))
                    .and_then(|public| public.get(name.basename.as_ref()))
                    .into_iter()
            })
            .flatten()
            .filter(|f| Self::fits(ctx, **f, arguments, source));

        if let Some(found) = remote_matches.next() {
            return if remote_matches.next().is_some() {
                Err(FindFunctionError::Ambiguous)
            } else {
                Ok(*found)
            };
        }

        if name.namespace.is_empty() {
            let imported_namespaces = ctx.settings.imported_namespaces.iter();

            // TODO: CLEANUP: Clean up this code that gets the origin module of the callee
            let subject_module_fs_node_id =
                if let Some(first_type) = arguments.first().map(|arg| &arg.resolved_type) {
                    if let Ok(first_type) = ctx.resolved_ast.unalias(first_type) {
                        match &first_type.kind {
                            TypeKind::Structure(_, structure_ref) => Some(
                                ctx.resolved_ast
                                    .structures
                                    .get(*structure_ref)
                                    .expect("valid struct")
                                    .name
                                    .fs_node_id,
                            ),
                            TypeKind::Enum(_, enum_ref) => Some(
                                ctx.resolved_ast
                                    .enums
                                    .get(*enum_ref)
                                    .expect("valid enum")
                                    .name
                                    .fs_node_id,
                            ),
                            _ => None,
                        }
                    } else {
                        warn_type_alias_depth_exceeded(first_type);
                        None
                    }
                } else {
                    None
                }
                .into_iter();

            let mut matches = imported_namespaces
                .flat_map(|namespace| ctx.settings.namespace_to_dependency.get(namespace.as_ref()))
                .flatten()
                .flat_map(|dependency| ctx.settings.dependency_to_module.get(dependency))
                .copied()
                .chain(subject_module_fs_node_id)
                .unique()
                .flat_map(|module_fs_node_id| {
                    ctx.public_functions
                        .get(&module_fs_node_id)
                        .and_then(|public| public.get(name.basename.as_ref()))
                        .into_iter()
                        .flatten()
                })
                .filter(|f| Self::fits(ctx, **f, arguments, source));

            if let Some(found) = matches.next() {
                return if matches.next().is_some() {
                    Err(FindFunctionError::Ambiguous)
                } else {
                    Ok(*found)
                };
            }
        }

        Err(FindFunctionError::NotDefined)
    }

    pub fn find_near_matches(&self, ctx: &ResolveExprCtx, name: &Name) -> Vec<String> {
        let resolved_name = ResolvedName::new(self.fs_node_id, name);

        let local_matches = self.available.get(&resolved_name).into_iter().flatten();

        let remote_matches = (!name.namespace.is_empty())
            .then(|| {
                ctx.settings
                    .namespace_to_dependency
                    .get(name.namespace.as_ref())
            })
            .flatten()
            .into_iter()
            .flatten()
            .flat_map(|dependency| {
                ctx.settings
                    .dependency_to_module
                    .get(dependency)
                    .and_then(|module_fs_node_id| ctx.public_functions.get(module_fs_node_id))
                    .and_then(|public| public.get(name.basename.as_ref()))
                    .into_iter()
            })
            .flatten();

        local_matches
            .chain(remote_matches)
            .map(|function_ref| {
                let function = ctx.resolved_ast.functions.get(*function_ref).unwrap();

                format!(
                    "{}({})",
                    function.name.display(&ctx.resolved_ast.workspace.fs),
                    function.parameters
                )
            })
            .collect_vec()
    }

    fn fits(
        ctx: &ResolveExprCtx,
        function_ref: FunctionRef,
        arguments: &[TypedExpr],
        source: Source,
    ) -> bool {
        let function = ctx.resolved_ast.functions.get(function_ref).unwrap();
        let parameters = &function.parameters;

        if !parameters.is_cstyle_vararg && arguments.len() != parameters.required.len() {
            return false;
        }

        if arguments.len() < parameters.required.len() {
            return false;
        }

        for (i, argument) in arguments.iter().enumerate() {
            let preferred_type = (i < parameters.required.len())
                .then_some(PreferredType::of_parameter(function_ref, i));

            let argument_conform =
                if let Some(preferred_type) = preferred_type.map(|p| p.view(ctx.resolved_ast)) {
                    conform_expr::<Validate>(
                        ctx,
                        argument,
                        preferred_type,
                        ConformMode::ParameterPassing,
                        ctx.adept_conform_behavior(),
                        source,
                    )
                } else {
                    conform_expr_to_default::<Validate>(argument, ctx.c_integer_assumptions())
                };

            if argument_conform.is_err() {
                return false;
            }
        }

        true
    }
}
