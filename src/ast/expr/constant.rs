use super::Expr;

#[derive(Clone, Debug)]
pub struct ConstExpr {
    pub value: Expr,
}
