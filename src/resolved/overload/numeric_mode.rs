use crate::{
    ast::{CInteger, IntegerBits},
    ir::IntegerSign,
};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum NumericMode {
    Integer(IntegerSign),
    LooseIndeterminateSignInteger(CInteger),
    CheckOverflow(IntegerBits, IntegerSign),
    Float,
}
