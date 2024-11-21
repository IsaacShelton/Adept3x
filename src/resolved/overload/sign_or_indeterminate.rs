use crate::{ast::CInteger, ir::IntegerSign};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum SignOrIndeterminate {
    Sign(IntegerSign),
    Indeterminate(CInteger),
}
