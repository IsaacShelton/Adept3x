use super::Type;
use crate::ast::Expr;

#[derive(Clone, Debug)]
pub enum TypeArg {
    Type(Type),
    Expr(Expr),
}
