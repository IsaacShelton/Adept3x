use num::BigInt;
use primitives::IntegerRigidity;

#[derive(Clone, Debug)]
pub enum Integer {
    Known(Box<IntegerKnown>),
    Generic(BigInt),
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct IntegerKnown {
    pub rigidity: IntegerRigidity,
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
