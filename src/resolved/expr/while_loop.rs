use super::Expr;
use crate::resolved::Block;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct While {
    pub condition: Expr,
    pub block: Block,
}
