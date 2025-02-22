use super::{
    conform::{
        conform_expr, to_default::conform_expr_to_default, warn_type_alias_depth_exceeded,
        ConformMode, Validate,
    },
    expr::{PreferredType, ResolveExprCtx},
    polymorph::PolyCatalog,
};
use crate::{
    asg::{self, Callee, TypeKind, TypedExpr},
    name::{Name, ResolvedName},
    resolve::{conform::Perform, PolyValue},
    source_files::Source,
    workspace::fs::FsNodeId,
};
use itertools::Itertools;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct FuncHaystack {
    pub available: HashMap<ResolvedName, Vec<asg::FuncRef>>,
    pub imported_namespaces: Vec<Box<str>>,
    pub module_fs_node_id: FsNodeId,
}

#[derive(Clone, Debug)]
pub enum FindFunctionError {
    NotDefined,
    Ambiguous,
}

impl FuncHaystack {
    pub fn new(imported_namespaces: Vec<Box<str>>, module_fs_node_id: FsNodeId) -> Self {
        Self {
            available: Default::default(),
            imported_namespaces,
            module_fs_node_id,
        }
    }

    pub fn find(
        &self,
        ctx: &ResolveExprCtx,
        name: &Name,
        generics: &[PolyValue],
        arguments: &[TypedExpr],
        source: Source,
    ) -> Result<Callee, FindFunctionError> {
        self.find_local(ctx, name, generics, arguments, source)
            .or_else(|| self.find_remote(ctx, name, generics, arguments, source))
            .or_else(|| self.find_imported(ctx, name, generics, arguments, source))
            .unwrap_or(Err(FindFunctionError::NotDefined))
    }

    pub fn find_near_matches(&self, ctx: &ResolveExprCtx, name: &Name) -> Vec<String> {
        // TODO: Clean up this function

        let local_matches = self
            .available
            .get(&ResolvedName::new(self.module_fs_node_id, name))
            .into_iter()
            .chain(
                (self.module_fs_node_id != ctx.physical_fs_node_id)
                    .then(|| {
                        self.available
                            .get(&ResolvedName::new(ctx.physical_fs_node_id, name))
                    })
                    .into_iter()
                    .flatten(),
            )
            .flatten();

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
                    .and_then(|module_fs_node_id| ctx.public_funcs.get(module_fs_node_id))
                    .and_then(|public| public.get(name.basename.as_ref()))
                    .into_iter()
            })
            .flatten();

        local_matches
            .chain(remote_matches)
            .map(|func_ref| {
                let function = ctx.asg.funcs.get(*func_ref).unwrap();

                format!(
                    "{}({})",
                    function.name.display(&ctx.asg.workspace.fs),
                    function.params
                )
            })
            .collect_vec()
    }

    pub fn fits(
        ctx: &ResolveExprCtx,
        func_ref: asg::FuncRef,
        generics: &[PolyValue],
        args: &[TypedExpr],
        existing_catalog: Option<PolyCatalog>,
        source: Source,
    ) -> Option<Callee> {
        let function = ctx.asg.funcs.get(func_ref).unwrap();
        let params = &function.params;

        let mut catalog = existing_catalog.unwrap_or_default();

        if generics.len() > function.type_params.len() {
            return None;
        }

        for (name, poly_value) in function
            .type_params
            .names()
            .take(generics.len())
            .zip(generics.iter())
        {
            if catalog
                .polymorphs
                .insert(name.clone(), poly_value.clone())
                .is_some()
            {
                return None;
            }
        }

        if !params.is_cstyle_vararg && args.len() != params.required.len() {
            return None;
        }

        if args.len() < params.required.len() {
            return None;
        }

        for (i, arg) in args.iter().enumerate() {
            let preferred_type =
                (i < params.required.len()).then_some(PreferredType::of_parameter(func_ref, i));

            let argument_conforms =
                if let Some(param_type) = preferred_type.map(|p| p.view(ctx.asg)) {
                    if param_type.kind.contains_polymorph() {
                        let Ok(argument) =
                            conform_expr_to_default::<Perform>(arg, ctx.c_integer_assumptions())
                        else {
                            return None;
                        };

                        Self::conform_polymorph(ctx, &mut catalog, &argument, param_type)
                    } else {
                        conform_expr::<Validate>(
                            ctx,
                            &arg,
                            param_type,
                            ConformMode::ParameterPassing,
                            ctx.adept_conform_behavior(),
                            source,
                        )
                        .is_ok()
                    }
                } else {
                    conform_expr_to_default::<Validate>(arg, ctx.c_integer_assumptions()).is_ok()
                };

            if !argument_conforms {
                return None;
            }
        }

        Some(Callee {
            func_ref,
            recipe: catalog.bake(),
        })
    }

    pub fn conform_polymorph(
        ctx: &ResolveExprCtx,
        catalog: &mut PolyCatalog,
        argument: &TypedExpr,
        param_type: &asg::Type,
    ) -> bool {
        catalog
            .extend_if_match_type(ctx, param_type, &argument.ty)
            .is_ok()
    }

    fn find_local(
        &self,
        ctx: &ResolveExprCtx,
        name: &Name,
        generics: &[PolyValue],
        arguments: &[TypedExpr],
        source: Source,
    ) -> Option<Result<Callee, FindFunctionError>> {
        let mut local_matches = self
            .available
            .get(&ResolvedName::new(self.module_fs_node_id, name))
            .into_iter()
            .chain(
                (self.module_fs_node_id != ctx.physical_fs_node_id)
                    .then(|| {
                        self.available
                            .get(&ResolvedName::new(ctx.physical_fs_node_id, name))
                    })
                    .into_iter()
                    .flatten(),
            )
            .flatten()
            .flat_map(|f| Self::fits(ctx, *f, generics, arguments, None, source));

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
        generics: &[PolyValue],
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
                    .and_then(|module_fs_node_id| ctx.public_funcs.get(module_fs_node_id))
                    .and_then(|public| public.get(name.basename.as_ref()))
                    .into_iter()
            })
            .flatten()
            .flat_map(|f| Self::fits(ctx, *f, generics, arguments, None, source));

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
        generics: &[PolyValue],
        arguments: &[TypedExpr],
        source: Source,
    ) -> Option<Result<Callee, FindFunctionError>> {
        if !name.namespace.is_empty() {
            return None;
        }

        let subject_module = arguments
            .first()
            .map(|arg| &arg.ty)
            .and_then(|first_type| {
                if let Ok(first_type) = ctx.asg.unalias(first_type) {
                    match &first_type.kind {
                        TypeKind::Structure(_, struct_ref, _) => Some(
                            ctx.asg
                                .structs
                                .get(*struct_ref)
                                .expect("valid struct")
                                .name
                                .fs_node_id,
                        ),
                        TypeKind::Enum(_, enum_ref) => Some(
                            ctx.asg
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
                ctx.public_funcs
                    .get(&module_fs_node_id)
                    .and_then(|public| public.get(name.basename.as_ref()))
                    .into_iter()
                    .flatten()
            })
            .flat_map(|f| Self::fits(ctx, *f, generics, arguments, None, source));

        matches.next().map(|found| {
            if matches.next().is_some() {
                Err(FindFunctionError::Ambiguous)
            } else {
                Ok(found)
            }
        })
    }
}
