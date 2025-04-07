use crate::{Expr, Type};

#[derive(Clone, Debug)]
pub struct FixedArray {
    pub ast_type: Type,
    pub count: Expr,
}
