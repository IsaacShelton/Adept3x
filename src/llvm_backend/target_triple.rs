use super::BackendError;
use llvm_sys::{
    core::LLVMDisposeMessage,
    target_machine::{LLVMGetDefaultTargetTriple, LLVMGetTargetFromTriple, LLVMTargetRef},
};
use std::{
    ffi::{CStr, CString},
    mem::MaybeUninit,
};

pub unsafe fn get_triple() -> CString {
    return CString::from_raw(LLVMGetDefaultTargetTriple());
}

pub unsafe fn get_target_from_triple(triple: &CStr) -> Result<LLVMTargetRef, BackendError> {
    let mut target: MaybeUninit<LLVMTargetRef> = MaybeUninit::zeroed();
    let mut error_message: MaybeUninit<*mut i8> = MaybeUninit::zeroed();

    if LLVMGetTargetFromTriple(
        triple.as_ptr(),
        target.as_mut_ptr(),
        error_message.as_mut_ptr(),
    ) != 0
    {
        let message = CStr::from_ptr(error_message.assume_init()).to_owned();
        LLVMDisposeMessage(error_message.assume_init());
        Err(message
            .into_string()
            .unwrap_or_else(|_| "Failed to get target triple for platform".into())
            .into())
    } else {
        Ok(target.assume_init())
    }
}
