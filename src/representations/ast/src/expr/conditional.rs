use crate::{Block, Expr};

#[derive(Clone, Debug)]
pub struct Conditional {
    pub conditions: Vec<(Expr, Block)>,
    pub otherwise: Option<Block>,
}
