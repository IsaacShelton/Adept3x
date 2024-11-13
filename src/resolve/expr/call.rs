use super::{resolve_expr, PreferredType, ResolveExprCtx};
use crate::{
    ast::{self, CInteger, FloatSize},
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
use num::BigInt;

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
            "bool" => Some(resolved::TypeKind::Boolean),
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

        let argument_type_kind = &arguments[0].resolved_type.kind;

        if let Some(target_type_kind) = target_type_kind {
            if target_type_kind.is_boolean() && argument_type_kind.is_integer_literal() {
                let argument = arguments.into_iter().next().unwrap();
                let is_initialized = argument.is_initialized;

                let resolved::ExprKind::IntegerLiteral(value) = &argument.expr.kind else {
                    unreachable!();
                };

                return Ok(TypedExpr {
                    resolved_type: target_type_kind.at(source),
                    expr: resolved::ExprKind::BooleanLiteral(*value != BigInt::ZERO).at(source),
                    is_initialized,
                });
            }

            if target_type_kind.is_boolean()
                && (argument_type_kind.is_integer_like() || argument_type_kind.is_float_like())
            {
                let target_type = target_type_kind.at(source);
                let argument = arguments.into_iter().next().unwrap();
                let is_initialized = argument.is_initialized;

                let expr = resolved::ExprKind::UnaryMathOperation(Box::new(
                    resolved::UnaryMathOperation {
                        operator: resolved::UnaryMathOperator::IsNonZero,
                        inner: argument,
                    },
                ))
                .at(source);

                return Ok(TypedExpr {
                    resolved_type: target_type,
                    expr,
                    is_initialized,
                });
            }

            if argument_type_kind.is_floating() {
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

            if argument_type_kind.is_integer_like() || argument_type_kind.is_boolean() {
                let target_type = target_type_kind.at(source);
                let argument = arguments.into_iter().next().unwrap();

                let expr = resolved::ExprKind::IntegerCast(Box::new(CastFrom {
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

        let to_float = match name.as_ref() {
            "f32" | "float" => Some((resolved::TypeKind::f32(), FloatSize::Bits32)),
            "f64" | "double" => Some((resolved::TypeKind::f64(), FloatSize::Bits64)),
            _ => None,
        };

        if let Some((target_type_kind, float_size)) = to_float {
            if argument_type_kind.is_integer_literal() {
                let argument = arguments.into_iter().next().unwrap();
                let is_initialized = argument.is_initialized;

                let resolved::ExprKind::IntegerLiteral(value) = &argument.expr.kind else {
                    unreachable!();
                };

                // TOOD: CLEANUP: This conversion could probably be cleaner
                let Ok(value) = i64::try_from(value)
                    .map(|x| x as f64)
                    .or_else(|_| u64::try_from(value).map(|x| x as f64))
                    .or_else(|_| value.to_string().parse::<f64>())
                else {
                    return Err(ResolveErrorKind::Other {
                        message: format!("Cannot create out-of-range floating-point number"),
                    }
                    .at(source));
                };

                return Ok(TypedExpr {
                    resolved_type: target_type_kind.at(source),
                    expr: resolved::ExprKind::FloatingLiteral(float_size, value).at(source),
                    is_initialized,
                });
            }

            if argument_type_kind.is_float_literal() {
                let argument = arguments.into_iter().next().unwrap();
                let is_initialized = argument.is_initialized;

                let resolved::ExprKind::FloatingLiteral(_size, value) = &argument.expr.kind else {
                    unreachable!();
                };

                return Ok(TypedExpr {
                    resolved_type: target_type_kind.at(source),
                    expr: resolved::ExprKind::FloatingLiteral(float_size, *value).at(source),
                    is_initialized,
                });
            }

            if argument_type_kind.is_integer_like() || argument_type_kind.is_boolean() {
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

    let callee = match ctx
        .function_haystack
        .find(ctx, &call.function_name, &arguments[..], source)
    {
        Ok(function_ref) => function_ref,
        Err(reason) => {
            let args = arguments
                .iter()
                .map(|arg| arg.resolved_type.to_string())
                .collect_vec();

            let signature = format!("{}({})", call.function_name, args.join(", "));

            let almost_matches = ctx
                .function_haystack
                .find_near_matches(ctx, &call.function_name);

            return Err(ResolveErrorKind::FailedToFindFunction {
                signature,
                reason,
                almost_matches,
            }
            .at(source));
        }
    };

    let function = ctx.resolved_ast.functions.get(callee.function).unwrap();
    let return_type = function.return_type.clone();

    let num_required = function.parameters.required.len();

    for (i, argument) in arguments.iter_mut().enumerate() {
        let function = ctx.resolved_ast.functions.get(callee.function).unwrap();

        let preferred_type =
            (i < num_required).then_some(PreferredType::of_parameter(callee.function, i));

        if preferred_type.map_or(false, |ty| {
            ty.view(&ctx.resolved_ast).kind.contains_polymorph()
        }) {
            // Skip, as already conformed
            continue;
        }

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
            resolved::ExprKind::Call(Box::new(resolved::Call { callee, arguments })),
            source,
        ),
    ))
}
