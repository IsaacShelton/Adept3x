mod compute;
mod integer_literals;

use super::{
    conform::{ConformMode, Perform, conform_expr},
    expr::ResolveExprCtx,
};
use asg::TypedExpr;
use ast::ConformBehavior;
use compute::compute_unifying_type;
use source_files::Source;

pub fn unify_types(
    ctx: &ResolveExprCtx,
    preferred_type: Option<&asg::Type>,
    exprs: &mut [&mut TypedExpr],
    behavior: ConformBehavior,
    conform_source: Source,
) -> Option<asg::Type> {
    // Compute the unifying type for the supplied expressions
    let unified_type = compute_unifying_type(preferred_type, exprs, behavior, conform_source)?;

    // Conform the supplied expressions if a unifying type was found
    for expr in exprs {
        **expr = match conform_expr::<Perform>(
            ctx,
            expr,
            &unified_type,
            ConformMode::Normal,
            behavior,
            conform_source,
        ) {
            Ok(conformed) => conformed,
            Err(_) => {
                panic!(
                    "cannot conform from '{}' to unified type '{unified_type}'",
                    expr.ty,
                );
            }
        }
    }

    Some(unified_type)
}
