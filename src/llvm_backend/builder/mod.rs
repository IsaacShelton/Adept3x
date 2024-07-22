mod gep;
mod gep_in_bounds;
mod gep_struct;
mod int_cast;
mod int_to_ptr;
mod load;
mod memcpy;
mod phi_relocation;
mod ptr_to_int;
mod store;

use std::ffi::CStr;

use append_only_vec::AppendOnlyVec;
use cstr::cstr;
use llvm_sys::{
    core::{LLVMBuildBitCast, LLVMCreateBuilder, LLVMDisposeBuilder},
    prelude::{LLVMBuilderRef, LLVMTypeRef, LLVMValueRef},
};
pub use load::Volatility;
pub use phi_relocation::PhiRelocation;

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

    pub fn bitcast(&self, value: LLVMValueRef, new_type: LLVMTypeRef) -> LLVMValueRef {
        self.bitcast_with_name(value, new_type, cstr!(""))
    }

    pub fn bitcast_with_name(
        &self,
        value: LLVMValueRef,
        new_type: LLVMTypeRef,
        name: &CStr,
    ) -> LLVMValueRef {
        unsafe { LLVMBuildBitCast(self.get(), value, new_type, name.as_ptr()) }
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
