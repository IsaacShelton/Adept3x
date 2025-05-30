use crate::{Block, Expr, Language};

#[derive(Clone, Debug)]
pub struct Conditional {
    pub conditions: Box<[(Expr, Block)]>,
    pub otherwise: Option<Block>,
    pub language: Language,
}
