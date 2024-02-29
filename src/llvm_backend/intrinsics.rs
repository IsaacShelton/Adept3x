use cstr::cstr;
use llvm_sys::{
    core::{
        LLVMAddFunction, LLVMFunctionType, LLVMInt1Type, LLVMInt64Type, LLVMInt8Type,
        LLVMPointerType, LLVMVoidType,
    },
    prelude::{LLVMBool, LLVMModuleRef, LLVMValueRef},
};
use std::ffi::c_uint;

use super::module::BackendModule;

pub struct Intrinsics {
    module: LLVMModuleRef,
    memcpy: Option<LLVMValueRef>,
    memset: Option<LLVMValueRef>,
    stacksave: Option<LLVMValueRef>,
    stackrestore: Option<LLVMValueRef>,
    va_start: Option<LLVMValueRef>,
    va_end: Option<LLVMValueRef>,
    va_copy: Option<LLVMValueRef>,
}

impl Intrinsics {
    pub unsafe fn new(module: &BackendModule) -> Self {
        Self {
            module: module.get(),
            memcpy: None,
            memset: None,
            stacksave: None,
            stackrestore: None,
            va_start: None,
            va_end: None,
            va_copy: None,
        }
    }

    pub unsafe fn memcpy(&mut self) -> LLVMValueRef {
        *self.memcpy.get_or_insert_with(|| {
            let mut parameter_types = [
                LLVMPointerType(LLVMInt8Type(), 0),
                LLVMPointerType(LLVMInt8Type(), 0),
                LLVMInt64Type(),
                LLVMInt1Type(),
            ];
            let return_type = LLVMVoidType();
            let is_var_arg = false;

            let signature = LLVMFunctionType(
                return_type,
                parameter_types.as_mut_ptr(),
                parameter_types.len() as c_uint,
                is_var_arg as LLVMBool,
            );

            LLVMAddFunction(
                self.module,
                cstr!("llvm.memcpy.p0i8.p0i8.i64").as_ptr(),
                signature,
            )
        })
    }
}
