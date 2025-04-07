use llvm_sys::{
    core::{LLVMDisposeModule, LLVMModuleCreateWithName},
    prelude::LLVMModuleRef,
};
use std::ffi::CStr;

pub struct BackendModule {
    module: LLVMModuleRef,
}

impl BackendModule {
    pub unsafe fn new(module_name: &CStr) -> Self {
        Self {
            module: LLVMModuleCreateWithName(module_name.as_ptr()),
        }
    }

    pub unsafe fn get(&self) -> LLVMModuleRef {
        self.module
    }
}

impl Drop for BackendModule {
    fn drop(&mut self) {
        unsafe {
            LLVMDisposeModule(self.module);
        }
    }
}

impl From<BackendModule> for LLVMModuleRef {
    fn from(value: BackendModule) -> Self {
        unsafe { value.get() }
    }
}
