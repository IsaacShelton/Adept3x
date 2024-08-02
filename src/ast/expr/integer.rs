use crate::{ast::IntegerFixedBits, ir::IntegerSign};
use num::BigInt;

#[derive(Clone, Debug)]
pub enum Integer {
    Known(Box<IntegerKnown>),
    Generic(BigInt),
}

#[derive(Clone, Debug)]
pub struct IntegerKnown {
    pub bits: IntegerFixedBits,
    pub sign: IntegerSign,
    pub value: BigInt,
}

impl Integer {
    pub fn value(&self) -> &BigInt {
        match self {
            Integer::Known(known) => &known.value,
            Integer::Generic(value) => value,
        }
    }
}
