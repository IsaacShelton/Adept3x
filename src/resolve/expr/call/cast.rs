use crate::{
    asg::{self, Cast, CastFrom, TypedExpr},
    ast::{self, CInteger, FloatSize},
    ir::IntegerSign,
    resolve::{
        conform::{conform_expr, ConformMode, Perform},
        destination::resolve_expr_to_destination,
        error::{ResolveError, ResolveErrorKind},
        expr::ResolveExprCtx,
    },
    source_files::Source,
};
use itertools::Itertools;
use num::BigInt;
use ordered_float::NotNan;

pub fn cast(
    ctx: &mut ResolveExprCtx,
    call: &ast::Call,
    args: Vec<TypedExpr>,
    source: Source,
) -> Result<Result<TypedExpr, Vec<TypedExpr>>, ResolveError> {
    if !call.generics.is_empty() {
        return Err(ResolveError::other(
            "Resolution of casting call with generics is not implemented yet",
            source,
        ));
    }

    if !call.name.namespace.is_empty() || args.len() != 1 {
        return Ok(Err(args));
    }

    let name = &call.name.basename;

    let target_type_kind = match name.as_ref() {
        "deref" => match args.into_iter().exactly_one() {
            Ok(arg) => {
                if let asg::TypeKind::Ptr(inner) = &arg.ty.kind {
                    return Ok(Ok(TypedExpr {
                        ty: inner.as_ref().clone(),
                        is_initialized: arg.is_initialized,
                        expr: asg::ExprKind::Dereference(Box::new(arg)).at(source),
                    }));
                }

                return Ok(Err(vec![arg]));
            }
            Err(args) => {
                return Ok(Err(args.collect()));
            }
        },
        "ptr" => match args.into_iter().exactly_one() {
            Ok(arg) => {
                let is_initialized = arg.is_initialized;
                let destination = resolve_expr_to_destination(arg)?;

                return Ok(Ok(TypedExpr {
                    ty: destination.ty.clone().pointer(source),
                    is_initialized,
                    expr: asg::ExprKind::AddressOf(Box::new(destination)).at(source),
                }));
            }
            Err(args) => {
                return Ok(Err(args.collect()));
            }
        },
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

    let argument_type_kind = &args[0].ty.kind;

    if let Some(target_type_kind) = target_type_kind {
        if argument_type_kind.is_integer_literal() || argument_type_kind.is_float_literal() {
            return conform_expr::<Perform>(
                ctx,
                &args[0],
                &target_type_kind.at(source),
                ConformMode::Explicit,
                ctx.adept_conform_behavior(),
                source,
            )
            .map(Ok)
            .map_err(|_| ResolveError::other("Cannot cast literal to unsuitable type", source));
        }

        if target_type_kind.is_boolean() && argument_type_kind.is_integer_literal() {
            let argument = args.into_iter().next().unwrap();
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
            let argument = args.into_iter().next().unwrap();
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
            let argument = args.into_iter().next().unwrap();

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
            let argument = args.into_iter().next().unwrap();

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
            let argument = args.into_iter().next().unwrap();
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
            let argument = args.into_iter().next().unwrap();
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
            let argument = args.into_iter().next().unwrap();

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

    Ok(Err(args))
}