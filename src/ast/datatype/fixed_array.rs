use super::Type;
use crate::ast::Expr;

#[derive(Clone, Debug)]
pub struct FixedArray {
    pub ast_type: Type,
    pub count: Expr,
}
