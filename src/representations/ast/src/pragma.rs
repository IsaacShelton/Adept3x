use crate::Expr;

#[derive(Clone, Debug)]
pub struct Pragma {
    pub name: Option<UseBinding<String>>,
    pub expr: Expr,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum UseBinding<T> {
    Name(T),
    Wildcard,
}
