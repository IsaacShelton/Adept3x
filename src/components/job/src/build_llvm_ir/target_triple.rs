use diagnostics::ErrorDiagnostic;
use llvm_sys::{
    core::LLVMDisposeMessage,
    target_machine::{LLVMGetDefaultTargetTriple, LLVMGetTargetFromTriple, LLVMTargetRef},
};
use std::{
    ffi::{CStr, CString},
    mem::MaybeUninit,
};
use target::{Target, TargetArch, TargetOs};

pub unsafe fn get_triple(target: &Target) -> Result<CString, ErrorDiagnostic> {
    let Some((arch, os)) = target.arch().zip(target.os()) else {
        return if target.is_host() {
            // Fallback to host target triple if unknown platform
            Ok(CString::from_raw(LLVMGetDefaultTargetTriple()))
        } else {
            Err(ErrorDiagnostic::plain(format!(
                "Unsupported target {}",
                target
            )))
        };
    };

    Ok(match arch {
        TargetArch::X86_64 => match os {
            TargetOs::Windows => c"x86_64-pc-windows-gnu",
            TargetOs::Mac => c"x86_64-apple-darwin",
            TargetOs::Linux => c"x86_64-unknown-linux-gnu",
            TargetOs::FreeBsd => c"x86_64-unknown-freebsd",
        },
        TargetArch::Aarch64 => match os {
            TargetOs::Windows => c"aarch64-pc-windows-gnu",
            TargetOs::Mac => c"arm64-apple-darwin",
            TargetOs::Linux => c"aarch64-unknown-linux-gnu",
            TargetOs::FreeBsd => c"aarch64-unknown-freebsd",
        },
    }
    .into())
}

pub unsafe fn make_llvm_target(triple: &CStr) -> Result<LLVMTargetRef, ErrorDiagnostic> {
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
        Err(ErrorDiagnostic::plain(
            message
                .into_string()
                .unwrap_or_else(|_| "Failed to get target triple for platform".into()),
        ))
    } else {
        Ok(target.assume_init())
    }
}
