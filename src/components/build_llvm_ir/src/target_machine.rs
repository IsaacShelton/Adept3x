use llvm_sys::target_machine::{
    LLVMCodeGenOptLevel, LLVMCodeModel, LLVMCreateTargetMachine, LLVMDisposeTargetMachine,
    LLVMRelocMode, LLVMTargetMachineRef, LLVMTargetRef,
};
use std::ffi::CStr;

pub struct TargetMachine {
    target_machine: LLVMTargetMachineRef,
}

impl TargetMachine {
    pub unsafe fn new(
        target: LLVMTargetRef,
        triple: &CStr,
        cpu: &CStr,
        features: &CStr,
        level: LLVMCodeGenOptLevel,
        reloc: LLVMRelocMode,
        code_model: LLVMCodeModel,
    ) -> Self {
        Self {
            target_machine: LLVMCreateTargetMachine(
                target,
                triple.as_ptr(),
                cpu.as_ptr(),
                features.as_ptr(),
                level,
                reloc,
                code_model,
            ),
        }
    }

    pub unsafe fn get(&self) -> LLVMTargetMachineRef {
        self.target_machine
    }
}

impl Drop for TargetMachine {
    fn drop(&mut self) {
        unsafe { LLVMDisposeTargetMachine(self.target_machine) };
    }
}

impl From<TargetMachine> for LLVMTargetMachineRef {
    fn from(value: TargetMachine) -> Self {
        unsafe { value.get() }
    }
}
