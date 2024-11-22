use super::{
    conform::{
        conform_expr, to_default::conform_expr_to_default, warn_type_alias_depth_exceeded,
        ConformMode, Validate,
    },
    expr::{PreferredType, ResolveExprCtx},
};
use crate::{
    name::{Name, ResolvedName},
    resolve::conform::Perform,
    resolved::{self, Callee, PolyCatalog, TypeKind, TypedExpr},
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
    ) -> Result<Callee, FindFunctionError> {
        let resolved_name = ResolvedName::new(self.fs_node_id, name);

        self.find_local(ctx, &resolved_name, arguments, source)
            .or_else(|| self.find_remote(ctx, &name, arguments, source))
            .or_else(|| self.find_imported(ctx, &name, arguments, source))
            .unwrap_or(Err(FindFunctionError::NotDefined))
    }

    pub fn find_near_matches(&self, ctx: &ResolveExprCtx, name: &Name) -> Vec<String> {
        // TODO: Clean up this function

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
        function_ref: resolved::FunctionRef,
        arguments: &[TypedExpr],
        source: Source,
    ) -> Option<Callee> {
        let function = ctx.resolved_ast.functions.get(function_ref).unwrap();
        let parameters = &function.parameters;

        if !parameters.is_cstyle_vararg && arguments.len() != parameters.required.len() {
            return None;
        }

        if arguments.len() < parameters.required.len() {
            return None;
        }

        let mut catalog = PolyCatalog::new();

        for (i, argument) in arguments.iter().enumerate() {
            let preferred_type = (i < parameters.required.len())
                .then_some(PreferredType::of_parameter(function_ref, i));

            let argument_conforms = if let Some(param_type) =
                preferred_type.map(|p| p.view(ctx.resolved_ast))
            {
                if param_type.kind.contains_polymorph() {
                    // NOTE: We probably dont't want arguments to always conform to the default
                    // representation without taking the full function match into account, but
                    // this will work for now.
                    // This would require tracking each type match for each polymorph
                    // and then unifying afterward

                    let Ok(argument) =
                        conform_expr_to_default::<Perform>(argument, ctx.c_integer_assumptions())
                    else {
                        return None;
                    };

                    Self::conform_polymorph(&mut catalog, &argument, param_type)
                } else {
                    conform_expr::<Validate>(
                        ctx,
                        &argument,
                        param_type,
                        ConformMode::ParameterPassing,
                        ctx.adept_conform_behavior(),
                        source,
                    )
                    .is_ok()
                }
            } else {
                conform_expr_to_default::<Validate>(argument, ctx.c_integer_assumptions()).is_ok()
            };

            if !argument_conforms {
                return None;
            }
        }

        Some(Callee {
            function: function_ref,
            recipe: catalog.bake(),
        })
    }

    fn conform_polymorph(
        catalog: &mut PolyCatalog,
        argument: &TypedExpr,
        param_type: &resolved::Type,
    ) -> bool {
        catalog
            .match_type(param_type, &argument.resolved_type)
            .is_ok()
    }

    fn find_local(
        &self,
        ctx: &ResolveExprCtx,
        resolved_name: &ResolvedName,
        arguments: &[TypedExpr],
        source: Source,
    ) -> Option<Result<Callee, FindFunctionError>> {
        let mut local_matches = self
            .available
            .get(&resolved_name)
            .into_iter()
            .flatten()
            .flat_map(|f| Self::fits(ctx, *f, arguments, source));

        local_matches.next().map(|found| {
            if local_matches.next().is_some() {
                Err(FindFunctionError::Ambiguous)
            } else {
                Ok(found)
            }
        })
    }

    fn find_remote(
        &self,
        ctx: &ResolveExprCtx,
        name: &Name,
        arguments: &[TypedExpr],
        source: Source,
    ) -> Option<Result<Callee, FindFunctionError>> {
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
            .flat_map(|f| Self::fits(ctx, *f, arguments, source));

        remote_matches.next().map(|found| {
            if remote_matches.next().is_some() {
                Err(FindFunctionError::Ambiguous)
            } else {
                Ok(found)
            }
        })
    }

    fn find_imported(
        &self,
        ctx: &ResolveExprCtx,
        name: &Name,
        arguments: &[TypedExpr],
        source: Source,
    ) -> Option<Result<Callee, FindFunctionError>> {
        if !name.namespace.is_empty() {
            return None;
        }

        let subject_module = arguments
            .first()
            .map(|arg| &arg.resolved_type)
            .and_then(|first_type| {
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
            })
            .into_iter();

        let mut matches = ctx
            .settings
            .imported_namespaces
            .iter()
            .flat_map(|namespace| ctx.settings.namespace_to_dependency.get(namespace.as_ref()))
            .flatten()
            .flat_map(|dependency| ctx.settings.dependency_to_module.get(dependency))
            .copied()
            .chain(subject_module)
            .unique()
            .flat_map(|module_fs_node_id| {
                ctx.public_functions
                    .get(&module_fs_node_id)
                    .and_then(|public| public.get(name.basename.as_ref()))
                    .into_iter()
                    .flatten()
            })
            .flat_map(|f| Self::fits(ctx, *f, arguments, source));

        matches.next().map(|found| {
            if matches.next().is_some() {
                Err(FindFunctionError::Ambiguous)
            } else {
                Ok(found)
            }
        })
    }
}
