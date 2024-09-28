use super::{resolve_expr, PreferredType, ResolveExprCtx};
use crate::{
    ast,
    resolve::{
        conform::{conform_expr, to_default::conform_expr_to_default, ConformMode},
        error::{ResolveError, ResolveErrorKind},
        resolve_type, Initialized,
    },
    resolved::{self, TypedExpr},
    source_files::Source,
};

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

    let function_ref = match ctx.function_search_ctx.find_function(&call.function_name) {
        Ok(function_ref) => function_ref,
        Err(reason) => {
            return Err(ResolveErrorKind::FailedToFindFunction {
                name: call.function_name.to_string(),
                reason,
            }
            .at(source));
        }
    };

    let function = ctx.resolved_ast.functions.get(function_ref).unwrap();
    let return_type = function.return_type.clone();

    if let Some(required_ty) = &call.expected_to_return {
        let resolved_required_ty =
            resolve_type(ctx.type_search_ctx, required_ty, &mut Default::default())?;

        if resolved_required_ty != return_type {
            return Err(ResolveErrorKind::FunctionMustReturnType {
                of: required_ty.to_string(),
                function_name: function.name.to_string(),
            }
            .at(function.return_type.source));
        }
    }

    if call.arguments.len() < function.parameters.required.len() {
        return Err(ResolveErrorKind::NotEnoughArgumentsToFunction {
            name: function.name.to_string(),
        }
        .at(source));
    }

    let num_required = function.parameters.required.len();

    if call.arguments.len() > num_required && !function.parameters.is_cstyle_vararg {
        return Err(ResolveErrorKind::TooManyArgumentsToFunction {
            name: function.name.to_string(),
        }
        .at(source));
    }

    for (i, argument) in arguments.iter_mut().enumerate() {
        let function = ctx.resolved_ast.functions.get(function_ref).unwrap();

        let preferred_type =
            (i < num_required).then_some(PreferredType::of_parameter(function_ref, i));

        if let Some(preferred_type) =
            preferred_type.map(|preferred_type| preferred_type.view(ctx.resolved_ast))
        {
            if let Ok(conformed_argument) = conform_expr(
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
                    name: function.name.to_string(),
                    i,
                }
                .at(source));
            }
        } else {
            match conform_expr_to_default(argument.clone(), ctx.c_integer_assumptions()) {
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
