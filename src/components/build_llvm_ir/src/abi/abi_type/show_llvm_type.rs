use core::fmt::Debug;
use llvm_sys::{core::LLVMPrintTypeToString, prelude::LLVMTypeRef};
use std::ffi::CString;

pub struct ShowLLVMType(pub LLVMTypeRef);

impl Debug for ShowLLVMType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let representation = unsafe { CString::from_raw(LLVMPrintTypeToString(self.0)) };
        write!(f, "LLVMTypeRef::{}", representation.to_str().unwrap())
    }
}
