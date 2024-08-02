use super::Expr;

#[derive(Clone, Debug)]
pub struct ArrayAccess {
    pub subject: Expr,
    pub index: Expr,
}
