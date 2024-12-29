use super::Expr;
use crate::asg::Block;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct While {
    pub condition: Expr,
    pub block: Block,
}
