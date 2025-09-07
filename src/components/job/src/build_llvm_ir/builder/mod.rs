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

use super::ctx::BackendCtx;
use append_only_vec::AppendOnlyVec;
use llvm_sys::{
    core::{
        LLVMBuildBitCast, LLVMBuildBr, LLVMBuildCall2, LLVMBuildZExt, LLVMCreateBuilder,
        LLVMDisposeBuilder, LLVMGetInsertBlock, LLVMPositionBuilderAtEnd,
    },
    prelude::{LLVMBasicBlockRef, LLVMBuilderRef, LLVMTypeRef, LLVMValueRef},
};
pub use load::Volatility;
pub use phi_relocation::PhiRelocation;
use std::{ffi::CStr, ptr::null_mut};

pub struct Builder<'env> {
    builder: LLVMBuilderRef,
    phi_relocations: AppendOnlyVec<PhiRelocation<'env>>,
}

impl<'env> Builder<'env> {
    pub unsafe fn new() -> Self {
        Self {
            builder: LLVMCreateBuilder(),
            phi_relocations: AppendOnlyVec::new(),
        }
    }

    pub unsafe fn get(&self) -> LLVMBuilderRef {
        self.builder
    }

    pub fn add_phi_relocation(&self, phi_relocation: PhiRelocation<'env>) {
        self.phi_relocations.push(phi_relocation);
    }

    pub fn take_phi_relocations(&mut self) -> AppendOnlyVec<PhiRelocation<'env>> {
        std::mem::replace(&mut self.phi_relocations, AppendOnlyVec::new())
    }

    pub fn zext(&self, value: LLVMValueRef, new_type: LLVMTypeRef) -> LLVMValueRef {
        unsafe { LLVMBuildZExt(self.get(), value, new_type, c"".as_ptr()) }
    }

    pub fn zext_with_name(
        &self,
        value: LLVMValueRef,
        new_type: LLVMTypeRef,
        name: &CStr,
    ) -> LLVMValueRef {
        unsafe { LLVMBuildZExt(self.get(), value, new_type, name.as_ptr()) }
    }

    pub fn bitcast(&self, value: LLVMValueRef, new_type: LLVMTypeRef) -> LLVMValueRef {
        self.bitcast_with_name(value, new_type, c"")
    }

    pub fn bitcast_with_name(
        &self,
        value: LLVMValueRef,
        new_type: LLVMTypeRef,
        name: &CStr,
    ) -> LLVMValueRef {
        unsafe { LLVMBuildBitCast(self.get(), value, new_type, name.as_ptr()) }
    }

    pub fn current_block(&self) -> LLVMBasicBlockRef {
        unsafe { LLVMGetInsertBlock(self.get()) }
    }

    pub fn position(&self, basicblock: LLVMBasicBlockRef) {
        unsafe { LLVMPositionBuilderAtEnd(self.get(), basicblock) }
    }

    pub fn br(&self, basicblock: LLVMBasicBlockRef) -> LLVMValueRef {
        unsafe { LLVMBuildBr(self.get(), basicblock) }
    }

    pub fn save_stack_pointer(&self, ctx: &BackendCtx) -> LLVMValueRef {
        let (function, signature) = ctx.intrinsics.stacksave();

        unsafe { LLVMBuildCall2(self.get(), signature, function, null_mut(), 0, c"".as_ptr()) }
    }

    pub fn restore_stack_pointer(&self, ctx: &BackendCtx, stack_pointer: LLVMValueRef) {
        let (function, signature) = ctx.intrinsics.stackrestore();
        let mut args = [stack_pointer];

        unsafe {
            LLVMBuildCall2(
                self.get(),
                signature,
                function,
                args.as_mut_ptr(),
                args.len() as _,
                c"".as_ptr(),
            );
        }
    }
}

impl<'env> Drop for Builder<'env> {
    fn drop(&mut self) {
        unsafe { LLVMDisposeBuilder(self.builder) };
    }
}

impl<'env> From<Builder<'env>> for LLVMBuilderRef {
    fn from(value: Builder) -> Self {
        unsafe { value.get() }
    }
}
