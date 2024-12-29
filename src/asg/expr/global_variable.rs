use crate::asg::{GlobalVarRef, Type};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct GlobalVariable {
    pub reference: GlobalVarRef,
    pub ty: Type,
}
