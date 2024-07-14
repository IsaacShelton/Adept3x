use super::target_machine::TargetMachine;
use llvm_sys::{
    target::{LLVMDisposeTargetData, LLVMTargetDataRef},
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

    pub unsafe fn get(&self) -> LLVMTargetDataRef {
        self.target_data
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
