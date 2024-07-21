use super::Builder;
use cstr::cstr;
use llvm_sys::{
    core::LLVMBuildPtrToInt,
    prelude::{LLVMTypeRef, LLVMValueRef},
};

impl Builder {
    pub fn ptr_to_int(&self, value: LLVMValueRef, integer_type: LLVMTypeRef) -> LLVMValueRef {
        unsafe { LLVMBuildPtrToInt(self.get(), value, integer_type, cstr!("").as_ptr()) }
    }
}
