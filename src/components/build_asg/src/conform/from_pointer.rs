use super::{ConformMode, Objective, ObjectiveResult, warn_type_alias_depth_exceeded};
use crate::{expr::ResolveExprCtx, unalias};
use asg::{Expr, Type, TypeKind, TypedExpr};

pub fn from_pointer<O: Objective>(
    ctx: &ResolveExprCtx,
    expr: &Expr,
    mode: ConformMode,
    from_inner_type: &Type,
    to_type: &Type,
) -> ObjectiveResult<O> {
    let Ok(from_inner_type) = unalias(ctx.asg, from_inner_type) else {
        warn_type_alias_depth_exceeded(from_inner_type);
        return O::fail();
    };

    let TypeKind::Ptr(to_inner_type) = &to_type.kind else {
        return O::fail();
    };

    let Ok(to_inner_type) = unalias(ctx.asg, to_inner_type) else {
        warn_type_alias_depth_exceeded(to_inner_type);
        return O::fail();
    };

    if from_inner_type.kind.is_void() {
        return O::success(|| TypedExpr::new(to_type.clone(), expr.clone()));
    }

    if to_inner_type.kind.is_void() && mode.allow_pointer_into_void_pointer() {
        return O::success(|| TypedExpr::new(to_type.clone(), expr.clone()));
    }

    O::fail()
}
