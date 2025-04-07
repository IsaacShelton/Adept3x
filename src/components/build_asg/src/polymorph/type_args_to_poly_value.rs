use crate::{
    error::ResolveError,
    expr::{ResolveExprCtx, ResolveExprMode, resolve_expr},
    initialized::Initialized,
    type_ctx::ResolveTypeOptions,
};
use asg::PolyValue;
use ast::TypeArg;

pub fn resolve_type_args_to_poly_args(
    ctx: &mut ResolveExprCtx,
    generics: &[TypeArg],
) -> Result<Vec<PolyValue>, ResolveError> {
    generics
        .iter()
        .map(|type_arg| match type_arg {
            TypeArg::Type(ty) => ctx
                .type_ctx()
                .resolve(ty, ResolveTypeOptions::Unalias)
                .map(PolyValue::Type),
            TypeArg::Expr(expr) => resolve_expr(
                ctx,
                expr,
                None,
                Initialized::Require,
                ResolveExprMode::RequireValue,
            )
            .map(PolyValue::Expr),
        })
        .collect()
}
