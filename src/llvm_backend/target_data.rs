use super::target_machine::TargetMachine;
use crate::data_units::ByteUnits;
use llvm_sys::{
    prelude::LLVMTypeRef,
    target::{
        LLVMABISizeOfType, LLVMDisposeTargetData, LLVMIntPtrType, LLVMStoreSizeOfType,
        LLVMTargetDataRef,
    },
    target_machine::LLVMCreateTargetDataLayout,
};

pub struct TargetData {
    target_data: LLVMTargetDataRef,
}

impl TargetData {
    pub unsafe fn new(target_machine: &TargetMachine) -> Self {
        Self {
            target_data: LLVMCreateTargetDataLayout(target_machine.get()),
        }
    }

    // SAFETY: It is the caller's responsibility to not use the returned LLVMTargetDataRef
    // after this goes out of scope
    pub unsafe fn get(&self) -> LLVMTargetDataRef {
        self.target_data
    }

    pub fn abi_size_of_type(&self, ty: LLVMTypeRef) -> ByteUnits {
        ByteUnits::of(unsafe { LLVMABISizeOfType(self.target_data, ty) })
    }

    pub fn store_size_of_type(&self, ty: LLVMTypeRef) -> ByteUnits {
        ByteUnits::of(unsafe { LLVMStoreSizeOfType(self.target_data, ty) })
    }

    pub fn pointer_sized_int_type(&self) -> LLVMTypeRef {
        unsafe { LLVMIntPtrType(self.get()) }
    }
}

impl Drop for TargetData {
    fn drop(&mut self) {
        unsafe { LLVMDisposeTargetData(self.target_data) };
    }
}

impl From<TargetData> for LLVMTargetDataRef {
    fn from(value: TargetData) -> Self {
        unsafe { value.get() }
    }
}
