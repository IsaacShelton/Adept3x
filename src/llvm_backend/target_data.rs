use super::target_machine::TargetMachine;
use llvm_sys::{
    prelude::LLVMTypeRef,
    target::{LLVMABISizeOfType, LLVMDisposeTargetData, LLVMIntPtrType, LLVMTargetDataRef},
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

    pub fn abi_size_of_type(&self, ty: LLVMTypeRef) -> usize {
        unsafe { LLVMABISizeOfType(self.target_data, ty) as usize }
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
