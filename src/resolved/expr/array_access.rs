use super::Expr;
use crate::resolved::Type;

#[derive(Clone, Debug)]
pub struct ArrayAccess {
    pub subject: Expr,
    pub item_type: Type,
    pub index: Expr,
}
