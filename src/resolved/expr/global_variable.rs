use crate::resolved::{GlobalVarRef, Type};

#[derive(Clone, Debug)]
pub struct GlobalVariable {
    pub reference: GlobalVarRef,
    pub resolved_type: Type,
}
