use super::{resolve_expr, PreferredType, ResolveExprCtx};
use crate::{
    ast::{self, CInteger},
    ir::IntegerSign,
    resolve::{
        conform::{conform_expr, to_default::conform_expr_to_default, ConformMode, Perform},
        error::{ResolveError, ResolveErrorKind},
        Initialized,
    },
    resolved::{self, Cast, CastFrom, TypedExpr},
    source_files::Source,
};
use itertools::Itertools;

pub fn resolve_call_expr(
    ctx: &mut ResolveExprCtx,
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

    // Capture primitive casts
    // TODO: CLEANUP: Clean up this code and add more types
    if call.function_name.namespace.is_empty() && arguments.len() == 1 {
        let name = &call.function_name.basename;

        let target_type_kind = match name.as_ref() {
            "u8" => Some(resolved::TypeKind::u8()),
            "u16" => Some(resolved::TypeKind::u16()),
            "u32" => Some(resolved::TypeKind::u32()),
            "u64" => Some(resolved::TypeKind::u64()),
            "i8" => Some(resolved::TypeKind::i8()),
            "i16" => Some(resolved::TypeKind::i16()),
            "i32" => Some(resolved::TypeKind::i32()),
            "i64" => Some(resolved::TypeKind::i64()),
            "char" => Some(resolved::TypeKind::CInteger(CInteger::Char, None)),
            "schar" => Some(resolved::TypeKind::CInteger(
                CInteger::Char,
                Some(IntegerSign::Signed),
            )),
            "uchar" => Some(resolved::TypeKind::CInteger(
                CInteger::Char,
                Some(IntegerSign::Unsigned),
            )),
            "short" => Some(resolved::TypeKind::CInteger(
                CInteger::Short,
                Some(IntegerSign::Signed),
            )),
            "ushort" => Some(resolved::TypeKind::CInteger(
                CInteger::Short,
                Some(IntegerSign::Unsigned),
            )),
            "int" => Some(resolved::TypeKind::CInteger(
                CInteger::Int,
                Some(IntegerSign::Signed),
            )),
            "uint" => Some(resolved::TypeKind::CInteger(
                CInteger::Int,
                Some(IntegerSign::Unsigned),
            )),
            "long" => Some(resolved::TypeKind::CInteger(
                CInteger::Long,
                Some(IntegerSign::Signed),
            )),
            "ulong" => Some(resolved::TypeKind::CInteger(
                CInteger::Long,
                Some(IntegerSign::Unsigned),
            )),
            "longlong" => Some(resolved::TypeKind::CInteger(
                CInteger::LongLong,
                Some(IntegerSign::Signed),
            )),
            "ulonglong" => Some(resolved::TypeKind::CInteger(
                CInteger::LongLong,
                Some(IntegerSign::Unsigned),
            )),
            _ => None,
        };

        if let Some(target_type_kind) = target_type_kind {
            if arguments[0].resolved_type.kind.is_floating() {
                let target_type = target_type_kind.at(source);
                let argument = arguments.into_iter().next().unwrap();

                let expr = resolved::ExprKind::FloatToInteger(Box::new(Cast {
                    target_type: target_type.clone(),
                    value: argument.expr,
                }))
                .at(source);

                return Ok(TypedExpr {
                    resolved_type: target_type,
                    expr,
                    is_initialized: argument.is_initialized,
                });
            }
        }

        let target_type_kind = match name.as_ref() {
            "f32" => Some(resolved::TypeKind::f32()),
            "f64" => Some(resolved::TypeKind::f64()),
            _ => None,
        };

        if let Some(target_type_kind) = target_type_kind {
            if arguments[0].resolved_type.kind.is_integer_like() {
                let target_type = target_type_kind.at(source);
                let argument = arguments.into_iter().next().unwrap();

                let expr = resolved::ExprKind::IntegerToFloat(Box::new(CastFrom {
                    cast: Cast {
                        target_type: target_type.clone(),
                        value: argument.expr,
                    },
                    from_type: argument.resolved_type,
                }))
                .at(source);

                return Ok(TypedExpr {
                    resolved_type: target_type,
                    expr,
                    is_initialized: argument.is_initialized,
                });
            }
        }
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
                ctx,
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
