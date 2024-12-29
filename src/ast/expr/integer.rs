use crate::{
    asg::{Type, TypeKind},
    ast::{CInteger, IntegerBits},
    ir::IntegerSign,
    source_files::Source,
};
use num::BigInt;

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

impl IntegerKnown {
    pub fn make_type(&self, source: Source) -> Type {
        match self.rigidity {
            IntegerRigidity::Fixed(bits, sign) => TypeKind::Integer(bits, sign),
            IntegerRigidity::Loose(c_integer, sign) => TypeKind::CInteger(c_integer, sign),
        }
        .at(source)
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum IntegerRigidity {
    Fixed(IntegerBits, IntegerSign),
    Loose(CInteger, Option<IntegerSign>),
}

impl Integer {
    pub fn value(&self) -> &BigInt {
        match self {
            Integer::Known(known) => &known.value,
            Integer::Generic(value) => value,
        }
    }
}
