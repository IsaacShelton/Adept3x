mod compute;
mod integer_literals;

use super::conform::{conform_expr, ConformMode};
use crate::{
    ast::ConformBehavior,
    resolved::{self, TypedExpr},
    source_files::Source,
};
use compute::compute_unifying_type;

pub fn unify_types(
    preferred_type: Option<&resolved::Type>,
    exprs: &mut [&mut TypedExpr],
    conform_behavior: ConformBehavior,
    conform_source: Source,
) -> Option<resolved::Type> {
    // Compute the unifying type for the supplied expressions
    let Some(unified_type) =
        compute_unifying_type(preferred_type, exprs, conform_behavior, conform_source)
    else {
        return None;
    };

    // Conform the supplied expressions if a unifying type was found
    for expr in exprs {
        **expr = match conform_expr(
            expr,
            &unified_type,
            ConformMode::Normal,
            ConformBehavior::Adept,
            conform_source,
        ) {
            Some(conformed) => conformed,
            None => {
                panic!(
                    "cannot conform to unified type {unified_type} for value of type {}",
                    expr.resolved_type,
                );
            }
        }
    }

    Some(unified_type)
}
