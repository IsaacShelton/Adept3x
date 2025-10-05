use crate::ir;
use llvm_sys::prelude::LLVMValueRef;

#[derive(Debug)]
pub struct PhiRelocation<'env> {
    pub phi_node: LLVMValueRef,
    pub incoming: &'env [ir::PhiIncoming<'env>],
}
