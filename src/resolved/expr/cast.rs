use super::Expr;
use crate::resolved::Type;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Cast {
    pub target_type: Type,
    pub value: Expr,
}

impl Cast {
    pub fn new(target_type: Type, value: Expr) -> Self {
        Self { target_type, value }
    }
}
