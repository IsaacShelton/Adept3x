use super::Expr;
use crate::resolved::Block;

#[derive(Clone, Debug)]
pub struct While {
    pub condition: Expr,
    pub block: Block,
}
