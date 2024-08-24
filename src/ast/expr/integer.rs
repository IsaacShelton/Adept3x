use crate::{
    ast::{CInteger, IntegerBits},
    ir::IntegerSign,
    resolved,
    source_files::Source,
};
use num::BigInt;

#[derive(Clone, Debug)]
pub enum Integer {
    Known(Box<IntegerKnown>),
    Generic(BigInt),
}

#[derive(Clone, Debug)]
pub struct IntegerKnown {
    pub rigidity: IntegerRigidity,
    pub value: BigInt,
    pub sign: IntegerSign,
}

impl IntegerKnown {
    pub fn make_type(&self, source: Source) -> resolved::Type {
        match self.rigidity {
            IntegerRigidity::Fixed(bits) => resolved::TypeKind::Integer(bits, self.sign),
            IntegerRigidity::Loose(c_integer) => {
                resolved::TypeKind::CInteger(c_integer, Some(self.sign))
            }
        }
        .at(source)
    }
}

#[derive(Clone, Debug)]
pub enum IntegerRigidity {
    Fixed(IntegerBits),
    Loose(CInteger),
}

impl Integer {
    pub fn value(&self) -> &BigInt {
        match self {
            Integer::Known(known) => &known.value,
            Integer::Generic(value) => value,
        }
    }
}
