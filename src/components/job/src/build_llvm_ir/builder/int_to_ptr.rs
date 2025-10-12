use super::Builder;
use llvm_sys::{
    core::LLVMBuildIntToPtr,
    prelude::{LLVMTypeRef, LLVMValueRef},
};

impl<'env> Builder<'env> {
    pub fn int_to_ptr(&mut self, value: LLVMValueRef, integer_type: LLVMTypeRef) -> LLVMValueRef {
        unsafe { LLVMBuildIntToPtr(self.get(), value, integer_type, c"".as_ptr()) }
    }
}
