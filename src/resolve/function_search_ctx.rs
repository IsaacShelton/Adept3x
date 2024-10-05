use super::{
    conform::{conform_expr, to_default::conform_expr_to_default, ConformMode, Validate},
    expr::{PreferredType, ResolveExprCtx},
};
use crate::{
    ir::FunctionRef,
    name::{Name, ResolvedName},
    resolved::{self, TypedExpr},
    source_files::Source,
};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct FunctionSearchCtx {
    pub available: HashMap<ResolvedName, Vec<resolved::FunctionRef>>,
    pub imported_namespaces: Vec<Box<str>>,
}

#[derive(Clone, Debug)]
pub enum FindFunctionError {
    NotDefined,
    Ambiguous,
}

impl FunctionSearchCtx {
    pub fn new(imported_namespaces: Vec<Box<str>>) -> Self {
        Self {
            available: Default::default(),
            imported_namespaces,
        }
    }

    pub fn find_function(
        &self,
        ctx: &ResolveExprCtx,
        name: &Name,
        arguments: &[TypedExpr],
        source: Source,
    ) -> Result<FunctionRef, FindFunctionError> {
        let resolved_name = ResolvedName::new(name);

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
                    .and_then(|module_fs_node_id| ctx.public.get(module_fs_node_id))
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
            let mut matches = self
                .imported_namespaces
                .iter()
                .filter_map(|namespace| {
                    self.available.get(&ResolvedName::new(&Name::new(
                        Some(namespace.to_string()),
                        name.basename.clone(),
                    )))
                })
                .flatten()
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
