use super::Expr;
use crate::asg::Type;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct TypedExpr {
    pub ty: Type,
    pub expr: Expr,
    pub is_initialized: bool,
}

impl TypedExpr {
    pub fn new(ty: Type, expr: Expr) -> Self {
        Self {
            ty,
            expr,
            is_initialized: true,
        }
    }

    pub fn new_maybe_initialized(ty: Type, expr: Expr, is_initialized: bool) -> Self {
        Self {
            ty,
            expr,
            is_initialized,
        }
    }
}
