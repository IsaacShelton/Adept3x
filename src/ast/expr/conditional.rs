use super::Expr;
use crate::ast::Block;

#[derive(Clone, Debug)]
pub struct Conditional {
    pub conditions: Vec<(Expr, Block)>,
    pub otherwise: Option<Block>,
}
