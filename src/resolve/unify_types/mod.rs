mod compute;
mod integer_literals;

use super::{conform_expr, ConformMode};
use crate::{
    ast::ConformBehavior,
    resolved::{self, TypedExpr},
    source_files::Source,
};
use compute::compute_unifying_type;

// bool <= u8 <= char <= u16 <= short <= int <= u32 <= long <= u64 <= longlong
//
//     - If any loose, result is loose
//     - Bits is max of min bits
//
//     - size_t is its own type, must be manually converted to/from
// error: 12345678 may not always fit inside `int`, which is only guaranteed to be 16-bits, use trunc<int>(1234243341343) to ignore

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
