use crate::{ast::CInteger, ir::IntegerSign};

#[derive(Clone, Debug)]
pub enum SignOrIndeterminate {
    Sign(IntegerSign),
    Indeterminate(CInteger),
}
