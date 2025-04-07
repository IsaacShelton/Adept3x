use super::{ConformMode, Objective, ObjectiveResult};
use asg::{
    Cast, CastFrom, Expr, ExprKind, Type, TypeKind, TypedExpr, UnaryMathOperation,
    UnaryMathOperator,
};
use ast::ConformBehavior;
use primitives::{CInteger, IntegerBits, IntegerSign};
use source_files::Source;

pub fn from_size_integer<O: Objective>(
    expr: &Expr,
    from_type: &Type,
    mode: ConformMode,
    behavior: ConformBehavior,
    from_sign: IntegerSign,
    to_type: &Type,
    source: Source,
) -> ObjectiveResult<O> {
    match &to_type.kind {
        TypeKind::Boolean => {
            from_size_integer_to_bool::<O>(expr, from_type, mode, behavior, source)
        }
        TypeKind::Integer(to_bits, to_sign) => {
            from_size_integer_to_integer::<O>(expr, mode, from_sign, *to_bits, *to_sign, source)
        }
        TypeKind::CInteger(to_c_integer, to_sign) => from_size_integer_to_c_integer::<O>(
            expr,
            mode,
            from_sign,
            *to_c_integer,
            *to_sign,
            source,
        ),
        _ => O::fail(),
    }
}

fn from_size_integer_to_bool<O: Objective>(
    expr: &Expr,
    from_type: &Type,
    mode: ConformMode,
    behavior: ConformBehavior,
    source: Source,
) -> ObjectiveResult<O> {
    if !behavior.auto_c_integer_to_bool_conversion() && !mode.allow_lossy_integer() {
        return O::fail();
    }

    O::success(|| {
        TypedExpr::new(
            TypeKind::Boolean.at(source),
            ExprKind::UnaryMathOperation(Box::new(UnaryMathOperation {
                operator: UnaryMathOperator::IsNonZero,
                inner: TypedExpr::new(from_type.clone(), expr.clone()),
            }))
            .at(source),
        )
    })
}

pub fn from_size_integer_to_c_integer<O: Objective>(
    expr: &Expr,
    mode: ConformMode,
    from_sign: IntegerSign,
    to_c_integer: CInteger,
    to_sign: Option<IntegerSign>,
    source: Source,
) -> ObjectiveResult<O> {
    if !(mode.allow_lossless_integer() && mode.allow_lossy_integer()) {
        return O::fail();
    }

    let target_type = TypeKind::CInteger(to_c_integer, to_sign).at(source);

    return O::success(|| {
        TypedExpr::new(
            target_type.clone(),
            ExprKind::IntegerCast(Box::new(CastFrom {
                cast: Cast::new(target_type, expr.clone()),
                from_type: TypeKind::SizeInteger(from_sign).at(source),
            }))
            .at(source),
        )
    });
}

fn from_size_integer_to_integer<O: Objective>(
    expr: &Expr,
    mode: ConformMode,
    from_sign: IntegerSign,
    to_bits: IntegerBits,
    to_sign: IntegerSign,
    source: Source,
) -> ObjectiveResult<O> {
    if !mode.allow_lossy_integer() {
        return O::fail();
    }

    let target_type = TypeKind::Integer(to_bits, to_sign).at(source);

    O::success(|| {
        TypedExpr::new(
            target_type.clone(),
            ExprKind::IntegerCast(Box::new(CastFrom {
                cast: Cast::new(target_type, expr.clone()),
                from_type: TypeKind::SizeInteger(from_sign).at(source),
            }))
            .at(source),
        )
    })
}
