use super::Type;
use crate::ast::Expr;

#[derive(Clone, Debug)]
pub enum CompileTimeArgument {
    Type(Type),
    Expr(Expr),
}
