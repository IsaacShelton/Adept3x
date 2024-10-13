use super::{resolve_expr, PreferredType, ResolveExprCtx};
use crate::{
    ast,
    resolve::{
        conform::{conform_expr, to_default::conform_expr_to_default, ConformMode, Perform},
        error::{ResolveError, ResolveErrorKind},
        Initialized,
    },
    resolved::{self, TypedExpr},
    source_files::Source,
};
use itertools::Itertools;

pub fn resolve_call_expr(
    ctx: &mut ResolveExprCtx<'_, '_>,
    call: &ast::Call,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    if !call.generics.is_empty() {
        return Err(ResolveErrorKind::Other {
            message: "Resolution of calls with generics is not implemented yet".into(),
        }
        .at(source));
    }

    let mut arguments = Vec::with_capacity(call.arguments.len());
    for argument in call.arguments.iter() {
        arguments.push(resolve_expr(ctx, argument, None, Initialized::Require)?);
    }

    let function_ref = match ctx.function_search_ctx.find_function(
        ctx,
        &call.function_name,
        &arguments[..],
        source,
    ) {
        Ok(function_ref) => function_ref,
        Err(reason) => {
            let args = arguments
                .iter()
                .map(|arg| arg.resolved_type.to_string())
                .collect_vec();

            let signature = format!("{}({})", call.function_name, args.join(", "));

            let almost_matches = ctx
                .function_search_ctx
                .find_function_almost_matches(ctx, &call.function_name);

            return Err(ResolveErrorKind::FailedToFindFunction {
                signature,
                reason,
                almost_matches,
            }
            .at(source));
        }
    };

    let function = ctx.resolved_ast.functions.get(function_ref).unwrap();
    let return_type = function.return_type.clone();

    let num_required = function.parameters.required.len();

    for (i, argument) in arguments.iter_mut().enumerate() {
        let function = ctx.resolved_ast.functions.get(function_ref).unwrap();

        let preferred_type =
            (i < num_required).then_some(PreferredType::of_parameter(function_ref, i));

        if let Some(preferred_type) =
            preferred_type.map(|preferred_type| preferred_type.view(ctx.resolved_ast))
        {
            if let Ok(conformed_argument) = conform_expr::<Perform>(
                &argument,
                preferred_type,
                ConformMode::ParameterPassing,
                ctx.adept_conform_behavior(),
                source,
            ) {
                *argument = conformed_argument;
            } else {
                return Err(ResolveErrorKind::BadTypeForArgumentToFunction {
                    expected: preferred_type.to_string(),
                    got: argument.resolved_type.to_string(),
                    name: function
                        .name
                        .display(&ctx.resolved_ast.workspace.fs)
                        .to_string(),
                    i,
                }
                .at(source));
            }
        } else {
            match conform_expr_to_default::<Perform>(&*argument, ctx.c_integer_assumptions()) {
                Ok(arg) => *argument = arg,
                Err(_) => {
                    return Err(ResolveErrorKind::Other {
                        message: "Failed to conform argument to default value".into(),
                    }
                    .at(source));
                }
            }
        }
    }

    if let Some(required_ty) = &call.expected_to_return {
        let resolved_required_ty = ctx.type_ctx().resolve(required_ty)?;

        if resolved_required_ty != return_type {
            return Err(ResolveErrorKind::FunctionMustReturnType {
                of: required_ty.to_string(),
                function_name: function
                    .name
                    .display(&ctx.resolved_ast.workspace.fs)
                    .to_string(),
            }
            .at(function.return_type.source));
        }
    }

    Ok(TypedExpr::new(
        return_type,
        resolved::Expr::new(
            resolved::ExprKind::Call(Box::new(resolved::Call {
                function: function_ref,
                arguments,
            })),
            source,
        ),
    ))
}
