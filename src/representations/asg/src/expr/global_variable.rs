use crate::{GlobalRef, Type};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct GlobalVariable {
    pub reference: GlobalRef,
    pub ty: Type,
}
