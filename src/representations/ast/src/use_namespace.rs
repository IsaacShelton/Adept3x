use crate::Expr;

#[derive(Clone, Debug)]
pub struct UseNamespace {
    pub name: Option<String>,
    pub expr: Expr,
}
