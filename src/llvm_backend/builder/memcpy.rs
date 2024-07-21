use super::Builder;
use crate::llvm_backend::address::Address;
use llvm_sys::{core::LLVMBuildMemCpy, prelude::LLVMValueRef};

impl Builder {
    pub fn memcpy(
        &self,
        destination: &Address,
        source: &Address,
        size: LLVMValueRef,
    ) -> LLVMValueRef {
        unsafe {
            LLVMBuildMemCpy(
                self.get(),
                destination.base_pointer(),
                destination.base.alignment.bytes().try_into().unwrap(),
                source.base_pointer(),
                source.base.alignment.bytes().try_into().unwrap(),
                size,
            )
        }
    }
}
