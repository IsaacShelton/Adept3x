use super::Expr;
use crate::{Destination, Type};
use derive_more::From;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct ArrayAccess {
    pub subject: ArrayDestination,
    pub item_type: Type,
    pub index: Expr,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, From)]
pub enum ArrayDestination {
    Expr(Expr),
    Destination(Destination),
}
