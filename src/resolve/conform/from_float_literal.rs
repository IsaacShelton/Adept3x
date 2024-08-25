use crate::{
    resolved::{Expr, ExprKind, Type, TypeKind, TypedExpr},
    source_files::Source,
};

pub fn from_float_literal(from: f64, to_type: &Type, source: Source) -> Option<TypedExpr> {
    match &to_type.kind {
        TypeKind::Floating(to_size) => Some(TypedExpr::new(
            TypeKind::Floating(*to_size).at(to_type.source),
            Expr::new(ExprKind::FloatingLiteral(*to_size, from), source),
        )),
        _ => None,
    }
}
