use super::{resolve_expr, PreferredType, ResolveExprCtx};
use crate::{
    ast::{self, ConformBehavior},
    resolve::{
        conform::{conform_expr, conform_expr_to_default, ConformMode},
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
    let function_ref = ctx
        .function_search_ctx
        .find_function_or_error(&call.function_name, source)?;

    let function = ctx.resolved_ast.functions.get(function_ref).unwrap();
    let return_type = function.return_type.clone();

    if let Some(required_ty) = &call.expected_to_return {
        let resolved_required_ty = resolve_type(
            ctx.type_search_ctx,
            ctx.resolved_ast.source_files,
            required_ty,
            &mut Default::default(),
        )?;

        if resolved_required_ty != return_type {
            return Err(ResolveErrorKind::FunctionMustReturnType {
                of: required_ty.to_string(),
                function_name: function.name.clone(),
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

    let mut arguments = Vec::with_capacity(call.arguments.len());

    for (i, argument) in call.arguments.iter().enumerate() {
        let preferred_type =
            (i < num_required).then_some(PreferredType::of_parameter(function_ref, i));

        let mut argument = resolve_expr(ctx, argument, preferred_type, Initialized::Require)?;

        let function = ctx.resolved_ast.functions.get(function_ref).unwrap();

        if let Some(preferred_type) =
            preferred_type.map(|preferred_type| preferred_type.view(ctx.resolved_ast))
        {
            if let Some(conformed_argument) = conform_expr(
                &argument,
                preferred_type,
                ConformMode::ParameterPassing,
                ConformBehavior::Adept,
                source,
            ) {
                argument = conformed_argument;
            } else {
                return Err(ResolveErrorKind::BadTypeForArgumentToFunction {
                    expected: preferred_type.to_string(),
                    got: argument.resolved_type.to_string(),
                    name: function.name.clone(),
                    i,
                }
                .at(source));
            }
        } else {
            match conform_expr_to_default(argument) {
                Ok(conformed_argument) => argument = conformed_argument,
                Err(error) => return Err(error),
            }
        }

        arguments.push(argument);
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
