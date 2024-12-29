use super::{Objective, ObjectiveResult};
use crate::{
    asg::{Expr, ExprKind, Type, TypeKind, TypedExpr},
    source_files::Source,
};
use ordered_float::NotNan;

pub fn from_float_literal<O: Objective>(
    from: Option<NotNan<f64>>,
    to_type: &Type,
    source: Source,
) -> ObjectiveResult<O> {
    match &to_type.kind {
        TypeKind::Floating(to_size) => O::success(|| {
            TypedExpr::new(
                TypeKind::Floating(*to_size).at(to_type.source),
                Expr::new(ExprKind::FloatingLiteral(*to_size, from), source),
            )
        }),
        _ => O::fail(),
    }
}
