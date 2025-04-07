use crate::{CInteger, IntegerBits, IntegerSign};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum IntegerRigidity {
    Fixed(IntegerBits, IntegerSign),
    Loose(CInteger, Option<IntegerSign>),
    Size(IntegerSign),
}
