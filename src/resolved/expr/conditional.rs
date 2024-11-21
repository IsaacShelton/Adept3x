use super::TypedExpr;
use crate::resolved::{Block, Type};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Conditional {
    pub result_type: Type,
    pub branches: Vec<Branch>,
    pub otherwise: Option<Block>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Branch {
    pub condition: TypedExpr,
    pub block: Block,
}
