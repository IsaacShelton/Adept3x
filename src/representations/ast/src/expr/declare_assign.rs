use super::Expr;

#[derive(Clone, Debug)]
pub struct DeclareAssign {
    pub name: String,
    pub value: Expr,
}
