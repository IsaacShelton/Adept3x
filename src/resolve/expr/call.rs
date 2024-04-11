use super::{resolve_expr, PreferredType, ResolveExprCtx};
use crate::{
    ast::{self, Source},
    resolve::{
        conform_expr, conform_expr_to_default,
        error::{ResolveError, ResolveErrorKind},
        Initialized,
    },
    resolved::{self, TypedExpr},
};

pub fn resolve_call_expr(
    ctx: &mut ResolveExprCtx<'_, '_>,
    call: &ast::Call,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    let function_ref = ctx
        .function_search_ctx
        .find_function_or_error(&call.function_name, source)?;

    let function = ctx.resolved_ast.functions.get(function_ref).unwrap();
    let return_type = function.return_type.clone();

    if call.arguments.len() < function.parameters.required.len() {
        return Err(ResolveError::new(
            ctx.resolved_ast.source_file_cache,
            source,
            ResolveErrorKind::NotEnoughArgumentsToFunction {
                name: function.name.to_string(),
            },
        ));
    }

    let num_required = function.parameters.required.len();

    if call.arguments.len() > num_required && !function.parameters.is_cstyle_vararg {
        return Err(ResolveError::new(
            ctx.resolved_ast.source_file_cache,
            source,
            ResolveErrorKind::TooManyArgumentsToFunction {
                name: function.name.to_string(),
            },
        ));
    }

    let mut arguments = Vec::with_capacity(call.arguments.len());

    for (i, argument) in call.arguments.iter().enumerate() {
        let preferred_type =
            (i < num_required).then_some(PreferredType::of_parameter(function_ref, i));

        let mut argument = resolve_expr(ctx, argument, preferred_type, Initialized::Require)?;

        let function = ctx.resolved_ast.functions.get(function_ref).unwrap();

        if let Some(preferred_type) =
            preferred_type.map(|preferred_type| preferred_type.view(ctx.resolved_ast))
        {
            if let Some(conformed_argument) = conform_expr(&argument, preferred_type) {
                argument = conformed_argument;
            } else {
                return Err(ResolveError::new(
                    ctx.resolved_ast.source_file_cache,
                    source,
                    ResolveErrorKind::BadTypeForArgumentToFunction {
                        expected: preferred_type.to_string(),
                        got: argument.resolved_type.to_string(),
                        name: function.name.clone(),
                        i,
                    },
                ));
            }
        } else {
            match conform_expr_to_default(argument, ctx.resolved_ast.source_file_cache) {
                Ok(conformed_argument) => argument = conformed_argument,
                Err(error) => return Err(error),
            }
        }

        arguments.push(argument.expr);
    }

    Ok(TypedExpr::new(
        return_type,
        resolved::Expr::new(
            resolved::ExprKind::Call(resolved::Call {
                function: function_ref,
                arguments,
            }),
            source,
        ),
    ))
}
