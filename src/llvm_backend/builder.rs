use llvm_sys::{
    core::{LLVMCreateBuilder, LLVMDisposeBuilder},
    prelude::LLVMBuilderRef,
};

pub struct Builder {
    builder: LLVMBuilderRef,
}

impl Builder {
    pub unsafe fn new() -> Self {
        Self {
            builder: LLVMCreateBuilder(),
        }
    }

    pub unsafe fn get(&self) -> LLVMBuilderRef {
        self.builder
    }
}

impl Drop for Builder {
    fn drop(&mut self) {
        unsafe { LLVMDisposeBuilder(self.builder) };
    }
}

impl From<Builder> for LLVMBuilderRef {
    fn from(value: Builder) -> Self {
        unsafe { value.get() }
    }
}
