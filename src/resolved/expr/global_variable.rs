use crate::resolved::{GlobalVarRef, Type};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct GlobalVariable {
    pub reference: GlobalVarRef,
    pub resolved_type: Type,
}
