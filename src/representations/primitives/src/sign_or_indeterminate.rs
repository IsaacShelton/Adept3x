use super::{CInteger, IntegerSign};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum SignOrIndeterminate {
    Sign(IntegerSign),
    Indeterminate(CInteger),
}
