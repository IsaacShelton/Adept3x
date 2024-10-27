use super::TypedExpr;
use crate::resolved::{Block, Type};

#[derive(Clone, Debug)]
pub struct Conditional {
    pub result_type: Type,
    pub branches: Vec<Branch>,
    pub otherwise: Option<Block>,
}

#[derive(Clone, Debug)]
pub struct Branch {
    pub condition: TypedExpr,
    pub block: Block,
}
