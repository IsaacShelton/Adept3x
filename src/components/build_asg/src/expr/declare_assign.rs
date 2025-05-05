use super::{ResolveExprCtx, ResolveExprMode, resolve_expr};
use crate::{
    conform::to_default::conform_expr_to_default_or_error,
    error::{ResolveError, ResolveErrorKind},
    initialized::Initialized,
};
use asg::TypedExpr;
use source_files::Source;

pub fn resolve_declare_assign_expr(
    ctx: &mut ResolveExprCtx<'_, '_>,
    declare_assign: &ast::DeclareAssign,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    let c_integer_assumptions = ctx.c_integer_assumptions();

    let value = conform_expr_to_default_or_error(
        resolve_expr(
            ctx,
            &declare_assign.value,
            None,
            Initialized::Require,
            ResolveExprMode::RequireValue,
        )?,
        c_integer_assumptions,
    )?;

    let Some(func_ref) = ctx.func_ref else {
        return Err(ResolveErrorKind::CannotDeclareVariableOutsideFunction.at(source));
    };

    let key = ctx.asg.funcs[func_ref].vars.add_variable(value.ty.clone());

    ctx.variable_haystack
        .put(&declare_assign.name, value.ty.clone(), key);

    Ok(TypedExpr::new(
        value.ty.clone(),
        asg::Expr::new(
            asg::ExprKind::DeclareAssign(Box::new(asg::DeclareAssign {
                key,
                value: value.expr,
                ty: value.ty,
            })),
            source,
        ),
    ))
}
