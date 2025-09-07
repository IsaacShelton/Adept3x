use super::Builder;
use llvm_sys::{
    core::LLVMBuildPtrToInt,
    prelude::{LLVMTypeRef, LLVMValueRef},
};

impl<'env> Builder<'env> {
    pub fn ptr_to_int(&self, value: LLVMValueRef, integer_type: LLVMTypeRef) -> LLVMValueRef {
        unsafe { LLVMBuildPtrToInt(self.get(), value, integer_type, c"".as_ptr()) }
    }
}
