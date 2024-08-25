use super::ConformMode;
use crate::resolved::{Type, TypeKind, TypedExpr};

pub fn from_pointer(
    expr: &TypedExpr,
    mode: ConformMode,
    from_inner: &Type,
    to_type: &Type,
) -> Option<TypedExpr> {
    if from_inner.kind.is_void() {
        return match &to_type.kind {
            TypeKind::Pointer(to_inner) => Some(TypedExpr::new(
                TypeKind::Pointer(to_inner.clone()).at(to_type.source),
                expr.expr.clone(),
            )),
            _ => None,
        };
    }

    if to_type.kind.is_void_pointer() && mode.allow_pointer_into_void_pointer() {
        return Some(TypedExpr::new(
            TypeKind::Pointer(Box::new(TypeKind::Void.at(to_type.source))).at(to_type.source),
            expr.expr.clone(),
        ));
    }

    None
}
