use append_only_vec::AppendOnlyVec;
use llvm_sys::{
    core::{LLVMCreateBuilder, LLVMDisposeBuilder},
    prelude::{LLVMBuilderRef, LLVMValueRef},
};

use crate::ir;

pub struct Builder {
    builder: LLVMBuilderRef,
    phi_relocations: AppendOnlyVec<PhiRelocation>,
}

impl Builder {
    pub unsafe fn new() -> Self {
        Self {
            builder: LLVMCreateBuilder(),
            phi_relocations: AppendOnlyVec::new(),
        }
    }

    pub unsafe fn get(&self) -> LLVMBuilderRef {
        self.builder
    }

    pub fn add_phi_relocation(&self, phi_relocation: PhiRelocation) {
        self.phi_relocations.push(phi_relocation);
    }

    pub fn take_phi_relocations(&mut self) -> AppendOnlyVec<PhiRelocation> {
        std::mem::replace(&mut self.phi_relocations, AppendOnlyVec::new())
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

pub struct PhiRelocation {
    pub phi_node: LLVMValueRef,
    pub incoming: Vec<ir::PhiIncoming>,
}
