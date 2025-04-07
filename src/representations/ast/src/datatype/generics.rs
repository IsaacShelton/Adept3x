use crate::{Expr, Type};

#[derive(Clone, Debug)]
pub enum TypeArg {
    Type(Type),
    Expr(Expr),
}
