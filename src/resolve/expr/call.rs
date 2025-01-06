use super::{resolve_expr, PreferredType, ResolveExprCtx, ResolveExprMode};
use crate::{
    asg::{self, Callee, Cast, CastFrom, TypedExpr},
    ast::{self, CInteger, FloatSize},
    ir::IntegerSign,
    resolve::{
        conform::{conform_expr, to_default::conform_expr_to_default, ConformMode, Perform},
        error::{ResolveError, ResolveErrorKind},
        Initialized,
    },
    source_files::Source,
};
use itertools::Itertools;
use num::BigInt;
use ordered_float::NotNan;

pub fn resolve_call_expr(
    ctx: &mut ResolveExprCtx,
    call: &ast::Call,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    if !call.generics.is_empty() {
        return Err(ResolveError::other(
            "Resolution of calls with generics is not implemented yet",
            source,
        ));
    }

    let mut args = Vec::with_capacity(call.args.len());
    for arg in call.args.iter() {
        args.push(resolve_expr(
            ctx,
            arg,
            None,
            Initialized::Require,
            ResolveExprMode::RequireValue,
        )?);
    }

    let args = match cast(ctx, call, args, source)? {
        Ok(cast) => return Ok(cast),
        Err(args) => args,
    };

    let callee = ctx
        .func_haystack
        .find(ctx, &call.name, &args[..], source)
        .map_err(|reason| {
            ResolveErrorKind::FailedToFindFunction {
                signature: format!(
                    "{}({})",
                    call.name,
                    args.iter().map(|arg| arg.ty.to_string()).join(", ")
                ),
                reason,
                almost_matches: ctx.func_haystack.find_near_matches(ctx, &call.name),
            }
            .at(source)
        })?;

    call_callee(ctx, call, callee, args, source)
}

pub fn call_callee(
    ctx: &mut ResolveExprCtx,
    call: &ast::Call,
    callee: Callee,
    mut args: Vec<TypedExpr>,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    let function = ctx.asg.funcs.get(callee.function).unwrap();
    let num_required = function.params.required.len();

    for (i, arg) in args.iter_mut().enumerate() {
        let function = ctx.asg.funcs.get(callee.function).unwrap();

        let preferred_type =
            (i < num_required).then_some(PreferredType::of_parameter(callee.function, i));

        if preferred_type.map_or(false, |ty| ty.view(&ctx.asg).kind.contains_polymorph()) {
            *arg = conform_expr_to_default::<Perform>(&*arg, ctx.c_integer_assumptions())
                .map_err(|_| ResolveErrorKind::FailedToConformArgumentToDefaultValue.at(source))?;
            continue;
        }

        *arg = preferred_type
            .map(|preferred_type| {
                let preferred_type = preferred_type.view(ctx.asg);
                conform_expr::<Perform>(
                    ctx,
                    &arg,
                    preferred_type,
                    ConformMode::ParameterPassing,
                    ctx.adept_conform_behavior(),
                    source,
                )
                .map_err(|_| {
                    ResolveErrorKind::BadTypeForArgumentToFunction {
                        expected: preferred_type.to_string(),
                        got: arg.ty.to_string(),
                        name: function.name.display(&ctx.asg.workspace.fs).to_string(),
                        i,
                    }
                    .at(source)
                })
            })
            .unwrap_or_else(|| {
                conform_expr_to_default::<Perform>(&*arg, ctx.c_integer_assumptions())
                    .map_err(|_| ResolveErrorKind::FailedToConformArgumentToDefaultValue.at(source))
            })?;
    }

    let return_type = callee
        .recipe
        .resolve_type(&function.return_type)
        .map_err(ResolveError::from)?;

    if let Some(required_ty) = &call.expected_to_return {
        let resolved_required_ty = ctx.type_ctx().resolve(required_ty)?;

        if resolved_required_ty != return_type {
            return Err(ResolveErrorKind::FunctionMustReturnType {
                of: required_ty.to_string(),
                func_name: function.name.display(&ctx.asg.workspace.fs).to_string(),
            }
            .at(function.return_type.source));
        }
    }

    Ok(TypedExpr::new(
        return_type,
        asg::Expr::new(
            asg::ExprKind::Call(Box::new(asg::Call {
                callee,
                arguments: args,
            })),
            source,
        ),
    ))
}

fn cast(
    ctx: &mut ResolveExprCtx,
    call: &ast::Call,
    arguments: Vec<TypedExpr>,
    source: Source,
) -> Result<Result<TypedExpr, Vec<TypedExpr>>, ResolveError> {
    if !call.generics.is_empty() {
        return Err(ResolveError::other(
            "Resolution of casting call with generics is not implemented yet",
            source,
        ));
    }

    if !call.name.namespace.is_empty() || arguments.len() != 1 {
        return Ok(Err(arguments));
    }

    let name = &call.name.basename;

    let target_type_kind = match name.as_ref() {
        "bool" => Some(asg::TypeKind::Boolean),
        "u8" => Some(asg::TypeKind::u8()),
        "u16" => Some(asg::TypeKind::u16()),
        "u32" => Some(asg::TypeKind::u32()),
        "u64" => Some(asg::TypeKind::u64()),
        "i8" => Some(asg::TypeKind::i8()),
        "i16" => Some(asg::TypeKind::i16()),
        "i32" => Some(asg::TypeKind::i32()),
        "i64" => Some(asg::TypeKind::i64()),
        "char" => Some(asg::TypeKind::CInteger(CInteger::Char, None)),
        "schar" => Some(asg::TypeKind::CInteger(
            CInteger::Char,
            Some(IntegerSign::Signed),
        )),
        "uchar" => Some(asg::TypeKind::CInteger(
            CInteger::Char,
            Some(IntegerSign::Unsigned),
        )),
        "short" => Some(asg::TypeKind::CInteger(
            CInteger::Short,
            Some(IntegerSign::Signed),
        )),
        "ushort" => Some(asg::TypeKind::CInteger(
            CInteger::Short,
            Some(IntegerSign::Unsigned),
        )),
        "int" => Some(asg::TypeKind::CInteger(
            CInteger::Int,
            Some(IntegerSign::Signed),
        )),
        "uint" => Some(asg::TypeKind::CInteger(
            CInteger::Int,
            Some(IntegerSign::Unsigned),
        )),
        "long" => Some(asg::TypeKind::CInteger(
            CInteger::Long,
            Some(IntegerSign::Signed),
        )),
        "ulong" => Some(asg::TypeKind::CInteger(
            CInteger::Long,
            Some(IntegerSign::Unsigned),
        )),
        "longlong" => Some(asg::TypeKind::CInteger(
            CInteger::LongLong,
            Some(IntegerSign::Signed),
        )),
        "ulonglong" => Some(asg::TypeKind::CInteger(
            CInteger::LongLong,
            Some(IntegerSign::Unsigned),
        )),
        _ => None,
    };

    let argument_type_kind = &arguments[0].ty.kind;

    if let Some(target_type_kind) = target_type_kind {
        if argument_type_kind.is_integer_literal() || argument_type_kind.is_float_literal() {
            return conform_expr::<Perform>(
                ctx,
                &arguments[0],
                &target_type_kind.at(source),
                ConformMode::Explicit,
                ctx.adept_conform_behavior(),
                source,
            )
            .map(Ok)
            .map_err(|_| ResolveError::other("Cannot cast literal to unsuitable type", source));
        }

        if target_type_kind.is_boolean() && argument_type_kind.is_integer_literal() {
            let argument = arguments.into_iter().next().unwrap();
            let is_initialized = argument.is_initialized;

            let asg::ExprKind::IntegerLiteral(value) = &argument.expr.kind else {
                unreachable!();
            };

            return Ok(Ok(TypedExpr {
                ty: target_type_kind.at(source),
                expr: asg::ExprKind::BooleanLiteral(*value != BigInt::ZERO).at(source),
                is_initialized,
            }));
        }

        if target_type_kind.is_boolean()
            && (argument_type_kind.is_integer_like() || argument_type_kind.is_float_like())
        {
            let target_type = target_type_kind.at(source);
            let argument = arguments.into_iter().next().unwrap();
            let is_initialized = argument.is_initialized;

            let expr = asg::ExprKind::UnaryMathOperation(Box::new(asg::UnaryMathOperation {
                operator: asg::UnaryMathOperator::IsNonZero,
                inner: argument,
            }))
            .at(source);

            return Ok(Ok(TypedExpr {
                ty: target_type,
                expr,
                is_initialized,
            }));
        }

        if argument_type_kind.is_floating() {
            let target_type = target_type_kind.at(source);
            let argument = arguments.into_iter().next().unwrap();

            let expr = asg::ExprKind::FloatToInteger(Box::new(Cast {
                target_type: target_type.clone(),
                value: argument.expr,
            }))
            .at(source);

            return Ok(Ok(TypedExpr {
                ty: target_type,
                expr,
                is_initialized: argument.is_initialized,
            }));
        }

        if argument_type_kind.is_integer_like() || argument_type_kind.is_boolean() {
            let target_type = target_type_kind.at(source);
            let argument = arguments.into_iter().next().unwrap();

            let expr = asg::ExprKind::IntegerCast(Box::new(CastFrom {
                cast: Cast {
                    target_type: target_type.clone(),
                    value: argument.expr,
                },
                from_type: argument.ty,
            }))
            .at(source);

            return Ok(Ok(TypedExpr {
                ty: target_type,
                expr,
                is_initialized: argument.is_initialized,
            }));
        }
    }

    let to_float = match name.as_ref() {
        "f32" | "float" => Some((asg::TypeKind::f32(), FloatSize::Bits32)),
        "f64" | "double" => Some((asg::TypeKind::f64(), FloatSize::Bits64)),
        _ => None,
    };

    if let Some((target_type_kind, float_size)) = to_float {
        if argument_type_kind.is_integer_literal() {
            let argument = arguments.into_iter().next().unwrap();
            let is_initialized = argument.is_initialized;

            let asg::ExprKind::IntegerLiteral(value) = &argument.expr.kind else {
                unreachable!();
            };

            // TOOD: CLEANUP: This conversion could probably be cleaner
            let Ok(value) = i64::try_from(value)
                .map(|x| x as f64)
                .or_else(|_| u64::try_from(value).map(|x| x as f64))
                .or_else(|_| value.to_string().parse::<f64>())
            else {
                return Err(ResolveErrorKind::CannotCreateOutOfRangeFloat.at(source));
            };

            return Ok(Ok(TypedExpr {
                ty: target_type_kind.at(source),
                expr: asg::ExprKind::FloatingLiteral(float_size, NotNan::new(value).ok())
                    .at(source),
                is_initialized,
            }));
        }

        if argument_type_kind.is_float_literal() {
            let argument = arguments.into_iter().next().unwrap();
            let is_initialized = argument.is_initialized;

            let asg::ExprKind::FloatingLiteral(_size, value) = &argument.expr.kind else {
                unreachable!();
            };

            return Ok(Ok(TypedExpr {
                ty: target_type_kind.at(source),
                expr: asg::ExprKind::FloatingLiteral(float_size, *value).at(source),
                is_initialized,
            }));
        }

        if argument_type_kind.is_integer_like() || argument_type_kind.is_boolean() {
            let target_type = target_type_kind.at(source);
            let argument = arguments.into_iter().next().unwrap();

            let expr = asg::ExprKind::IntegerToFloat(Box::new(CastFrom {
                cast: Cast {
                    target_type: target_type.clone(),
                    value: argument.expr,
                },
                from_type: argument.ty,
            }))
            .at(source);

            return Ok(Ok(TypedExpr {
                ty: target_type,
                expr,
                is_initialized: argument.is_initialized,
            }));
        }
    }

    Ok(Err(arguments))
}
