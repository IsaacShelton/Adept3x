use super::Expr;
use crate::ast::Block;

#[derive(Clone, Debug)]
pub struct While {
    pub condition: Expr,
    pub block: Block,
}
