use crate::{
    ast::FloatSize,
    resolved::{Cast, Expr, ExprKind, Type, TypeKind, TypedExpr},
    source_files::Source,
};

pub fn from_float(expr: &TypedExpr, from_size: FloatSize, to_type: &Type) -> Option<TypedExpr> {
    match &to_type.kind {
        TypeKind::Floating(to_size) => {
            from_float_to_float(&expr.expr, from_size, *to_size, to_type.source)
        }
        _ => None,
    }
}

fn from_float_to_float(
    expr: &Expr,
    from_size: FloatSize,
    to_size: FloatSize,
    type_source: Source,
) -> Option<TypedExpr> {
    let target_type = TypeKind::Floating(to_size).at(type_source);
    let from_bits = from_size.bits();
    let to_bits = to_size.bits();

    if from_bits == to_bits {
        return Some(TypedExpr::new(target_type, expr.clone()));
    }

    if from_bits < to_bits {
        return Some(TypedExpr::new(
            target_type.clone(),
            ExprKind::FloatExtend(Box::new(Cast::new(target_type, expr.clone()))).at(expr.source),
        ));
    }

    None
}
