use super::Builder;
use crate::{data_units::ByteUnits, llvm_backend::address::Address};
use llvm_sys::{
    core::{LLVMBuildStore, LLVMSetAlignment},
    prelude::LLVMValueRef,
};

impl Builder {
    pub fn store(&self, value: LLVMValueRef, destination: &Address) -> LLVMValueRef {
        self.store_aligned_raw(
            value,
            destination.base_pointer(),
            destination.base.alignment,
        )
    }

    pub fn store_raw(&self, value: LLVMValueRef, destination: LLVMValueRef) -> LLVMValueRef {
        unsafe { LLVMBuildStore(self.get(), value, destination) }
    }

    pub fn store_aligned_raw(
        &self,
        value: LLVMValueRef,
        destination: LLVMValueRef,
        alignment: ByteUnits,
    ) -> LLVMValueRef {
        let store = unsafe { LLVMBuildStore(self.get(), value, destination) };
        unsafe { LLVMSetAlignment(store, alignment.bytes().try_into().unwrap()) };
        store
    }
}
