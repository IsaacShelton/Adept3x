use crate::Expr;
use attributes::Privacy;

#[derive(Clone, Debug)]
pub struct Pragma {
    pub name: Option<UseBinding<String>>,
    pub expr: Expr,
    pub privacy: Privacy,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum UseBinding<T> {
    Name(T),
    Wildcard,
}
