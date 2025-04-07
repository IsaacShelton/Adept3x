use crate::{Block, Expr};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct While {
    pub condition: Expr,
    pub block: Block,
}
