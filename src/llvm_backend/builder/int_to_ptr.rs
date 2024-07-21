use super::Builder;
use cstr::cstr;
use llvm_sys::{
    core::LLVMBuildIntToPtr,
    prelude::{LLVMTypeRef, LLVMValueRef},
};

impl Builder {
    pub fn int_to_ptr(&self, value: LLVMValueRef, integer_type: LLVMTypeRef) -> LLVMValueRef {
        unsafe { LLVMBuildIntToPtr(self.get(), value, integer_type, cstr!("").as_ptr()) }
    }
}
