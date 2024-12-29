use super::{resolve_expr, ResolveExprCtx};
use crate::{
    ast::{self},
    resolve::{
        conform::to_default::conform_expr_to_default_or_error,
        error::{ResolveError, ResolveErrorKind},
        Initialized,
    },
    asg::{self, TypedExpr},
    source_files::Source,
};

pub fn resolve_declare_assign_expr(
    ctx: &mut ResolveExprCtx<'_, '_>,
    declare_assign: &ast::DeclareAssign,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    let c_integer_assumptions = ctx.c_integer_assumptions();

    let value = conform_expr_to_default_or_error(
        resolve_expr(ctx, &declare_assign.value, None, Initialized::Require)?,
        c_integer_assumptions,
    )?;

    let Some(resolved_function_ref) = ctx.resolved_function_ref else {
        return Err(ResolveErrorKind::CannotDeclareVariableOutsideFunction.at(source));
    };

    let function = ctx
        .asg
        .functions
        .get_mut(resolved_function_ref)
        .unwrap();

    let key = function
        .variables
        .add_variable(value.resolved_type.clone(), true);

    ctx.variable_haystack
        .put(&declare_assign.name, value.resolved_type.clone(), key);

    Ok(TypedExpr::new(
        value.resolved_type.clone(),
        asg::Expr::new(
            asg::ExprKind::DeclareAssign(Box::new(asg::DeclareAssign {
                key,
                value: value.expr,
                resolved_type: value.resolved_type,
            })),
            source,
        ),
    ))
}
