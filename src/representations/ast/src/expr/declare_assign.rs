use super::Expr;

#[derive(Clone, Debug)]
pub struct DeclareAssign {
    pub name: Box<str>,
    pub value: Expr,
}
