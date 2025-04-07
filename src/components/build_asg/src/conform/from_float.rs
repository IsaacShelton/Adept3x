use super::{ConformMode, Objective, ObjectiveResult};
use asg::{Cast, Expr, ExprKind, Type, TypeKind, TypedExpr};
use primitives::FloatSize;
use source_files::Source;

pub fn from_float<O: Objective>(
    expr: &Expr,
    mode: ConformMode,
    from_size: FloatSize,
    to_type: &Type,
) -> ObjectiveResult<O> {
    if !mode.allow_lossless_float() {
        return O::fail();
    }

    match &to_type.kind {
        TypeKind::Floating(to_size) => {
            from_float_to_float::<O>(&expr, from_size, *to_size, to_type.source)
        }
        _ => O::fail(),
    }
}

fn from_float_to_float<O: Objective>(
    expr: &Expr,
    from_size: FloatSize,
    to_size: FloatSize,
    type_source: Source,
) -> ObjectiveResult<O> {
    let target_type = TypeKind::Floating(to_size).at(type_source);
    let from_bits = from_size.bits();
    let to_bits = to_size.bits();

    if from_bits == to_bits {
        return O::success(|| TypedExpr::new(target_type, expr.clone()));
    }

    if from_bits < to_bits {
        return O::success(|| {
            TypedExpr::new(
                target_type.clone(),
                ExprKind::FloatExtend(Box::new(Cast::new(target_type, expr.clone())))
                    .at(expr.source),
            )
        });
    }

    O::fail()
}
