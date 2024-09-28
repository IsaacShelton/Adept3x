use super::{Objective, ObjectiveResult};
use crate::{
    resolved::{Expr, ExprKind, Type, TypeKind, TypedExpr},
    source_files::Source,
};

pub fn from_float_literal<O: Objective>(
    from: f64,
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
