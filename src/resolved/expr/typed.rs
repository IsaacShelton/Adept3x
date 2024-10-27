use super::Expr;
use crate::resolved::Type;

#[derive(Clone, Debug)]
pub struct TypedExpr {
    pub resolved_type: Type,
    pub expr: Expr,
    pub is_initialized: bool,
}

impl TypedExpr {
    pub fn new(resolved_type: Type, expr: Expr) -> Self {
        Self {
            resolved_type,
            expr,
            is_initialized: true,
        }
    }

    pub fn new_maybe_initialized(resolved_type: Type, expr: Expr, is_initialized: bool) -> Self {
        Self {
            resolved_type,
            expr,
            is_initialized,
        }
    }
}
