use super::{warn_type_alias_depth_exceeded, ConformMode, Objective, ObjectiveResult};
use crate::{
    asg::{Expr, Type, TypeKind, TypedExpr},
    resolve::expr::ResolveExprCtx,
};

pub fn from_pointer<O: Objective>(
    ctx: &ResolveExprCtx,
    expr: &Expr,
    mode: ConformMode,
    from_inner_type: &Type,
    to_type: &Type,
) -> ObjectiveResult<O> {
    let Ok(from_inner_type) = ctx.asg.unalias(from_inner_type) else {
        warn_type_alias_depth_exceeded(from_inner_type);
        return O::fail();
    };

    let TypeKind::Ptr(to_inner_type) = &to_type.kind else {
        return O::fail();
    };

    let Ok(to_inner_type) = ctx.asg.unalias(to_inner_type) else {
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
