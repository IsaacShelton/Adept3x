use crate::{Block, Type, TypedExpr};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Conditional {
    pub result_type: Option<Type>,
    pub branches: Vec<Branch>,
    pub otherwise: Option<Block>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Branch {
    pub condition: TypedExpr,
    pub block: Block,
}
