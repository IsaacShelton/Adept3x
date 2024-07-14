use crate::ir;
use llvm_sys::prelude::LLVMValueRef;

pub struct PhiRelocation {
    pub phi_node: LLVMValueRef,
    pub incoming: Vec<ir::PhiIncoming>,
}
