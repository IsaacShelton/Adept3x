use crate::{Expr, Type};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct TypedExpr {
    pub ty: Type,
    pub expr: Expr,
}

impl TypedExpr {
    pub fn new(ty: Type, expr: Expr) -> Self {
        Self { ty, expr }
    }
}
