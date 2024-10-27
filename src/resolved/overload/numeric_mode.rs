use crate::{
    ast::{CInteger, IntegerBits},
    ir::IntegerSign,
};

#[derive(Clone, Debug)]
pub enum NumericMode {
    Integer(IntegerSign),
    LooseIndeterminateSignInteger(CInteger),
    CheckOverflow(IntegerBits, IntegerSign),
    Float,
}
