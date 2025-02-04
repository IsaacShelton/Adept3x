use super::PolyValue;
use crate::{
    ast::TypeArg,
    resolve::{
        error::ResolveError,
        expr::{resolve_expr, ResolveExprCtx, ResolveExprMode},
        initialized::Initialized,
        type_ctx::ResolveTypeOptions,
    },
};

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
