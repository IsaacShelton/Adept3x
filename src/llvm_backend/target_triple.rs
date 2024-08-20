use super::BackendError;
use crate::target::{Target, TargetArch, TargetOs};
use cstr::cstr;
use llvm_sys::{
    core::LLVMDisposeMessage,
    target_machine::{LLVMGetDefaultTargetTriple, LLVMGetTargetFromTriple, LLVMTargetRef},
};
use std::{
    ffi::{CStr, CString},
    mem::MaybeUninit,
};

pub unsafe fn get_triple(target: &Target) -> Result<CString, BackendError> {
    let Some((arch, os)) = target.arch().zip(target.os()) else {
        return if target.is_host() {
            // Fallback to host target triple if unknown platform
            Ok(CString::from_raw(LLVMGetDefaultTargetTriple()))
        } else {
            Err(BackendError::plain(format!(
                "Unsupported target {}",
                target
            )))
        };
    };

    Ok(match arch {
        TargetArch::X86_64 => match os {
            TargetOs::Windows => cstr!("x86_64-pc-windows-gnu").into(),
            TargetOs::Mac => cstr!("x86_64-apple-darwin").into(),
            TargetOs::Linux => cstr!("x86_64-unknown-linux-gnu").into(),
            TargetOs::FreeBsd => cstr!("x86_64-unknown-freebsd").into(),
        },
        TargetArch::Aarch64 => match os {
            TargetOs::Windows => cstr!("aarch64-pc-windows-gnu").into(),
            TargetOs::Mac => cstr!("arm64-apple-darwin").into(),
            TargetOs::Linux => cstr!("aarch64-unknown-linux-gnu").into(),
            TargetOs::FreeBsd => cstr!("aarch64-unknown-freebsd").into(),
        },
    })
}

pub unsafe fn make_llvm_target(triple: &CStr) -> Result<LLVMTargetRef, BackendError> {
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
