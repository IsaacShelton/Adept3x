use super::Expr;
use crate::asg::Type;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct ArrayAccess {
    pub subject: Expr,
    pub item_type: Type,
    pub index: Expr,
}
