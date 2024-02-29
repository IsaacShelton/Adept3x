use llvm_sys::prelude::{LLVMTypeRef, LLVMValueRef};

pub struct VariableStack {
    pub variables: Vec<(LLVMValueRef, LLVMTypeRef)>,
}
