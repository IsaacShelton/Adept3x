use crate::{PolyCatalogExt, error::ResolveError, expr::ResolveExprCtx};
use asg::{IntoPolyRecipeResolver, PolyCatalog, PolyRecipeResolver, PolyValue};
use itertools::Itertools;
use source_files::Source;
use std::collections::HashSet;

pub fn infer_callee_missing_impl_args(
    ctx: &ResolveExprCtx,
    function: &asg::Func,
    used_names: &mut HashSet<String>,
    catalog: &mut PolyCatalog,
    source: Source,
) -> Result<(), ResolveError> {
    for (expected_name, expected_trait) in function.impl_params.iter() {
        if used_names.contains(expected_name) {
            continue;
        }

        let Some(caller) = ctx
            .func_ref
            .map(|caller_func_ref| ctx.asg.funcs.get(caller_func_ref).unwrap())
        else {
            continue;
        };

        let from_env = caller
            .impl_params
            .iter()
            .filter_map(|(param_name, param_trait)| {
                let matched = catalog
                    .try_match_all_types(ctx, &expected_trait.args, &param_trait.args)
                    .ok()?;

                PolyRecipeResolver::new_disjoint(&matched.partial, &catalog.resolver())
                    .resolve_trait(expected_trait)
                    .ok()
                    .and_then(|expected_trait| {
                        (param_trait.trait_ref == expected_trait.trait_ref)
                            .then_some((param_name, matched))
                    })
            });

        match from_env.exactly_one() {
            Ok((param_name, matched)) => {
                catalog.polymorphs.extend(matched.partial);

                if catalog
                    .polymorphs
                    .insert(expected_name.into(), PolyValue::PolyImpl(param_name.into()))
                    .is_some()
                {
                    return Err(ResolveError::other(
                        format!(
                            "Could not automatically supply trait implementation for '${} {}' required by function call, since the polymorph is already in use",
                            expected_name,
                            expected_trait.display(&ctx.asg),
                        ),
                        source,
                    ));
                }
            }
            Err(mut non_unique) => {
                return Err(ResolveError::other(
                    if non_unique.next().is_some() {
                        format!(
                            "Ambiguous trait implementation for '${} {}' required by function call, please specify manually",
                            expected_name,
                            expected_trait.display(&ctx.asg),
                        )
                    } else {
                        format!(
                            "Missing '${} {}' trait implementation required by function call",
                            expected_name,
                            expected_trait.display(&ctx.asg),
                        )
                    },
                    source,
                ));
            }
        }
    }

    Ok(())
}
