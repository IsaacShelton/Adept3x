use crate::{Block, ConformBehavior, Expr};

#[derive(Clone, Debug)]
pub struct Conditional {
    pub conditions: Box<[(Expr, Block)]>,
    pub otherwise: Option<Block>,
    pub conform_behavior: ConformBehavior,
}
