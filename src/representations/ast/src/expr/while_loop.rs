use crate::{Block, Expr};

#[derive(Clone, Debug)]
pub struct While {
    pub condition: Expr,
    pub block: Block,
}
