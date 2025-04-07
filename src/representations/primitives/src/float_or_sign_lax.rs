use super::{CInteger, IntegerSign};

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum FloatOrSignLax {
    Integer(IntegerSign),
    IndeterminateInteger(CInteger),
    Float,
}
