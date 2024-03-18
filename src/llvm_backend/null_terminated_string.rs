use cstr::cstr;
use llvm_sys::{
    core::{
        LLVMAddGlobal, LLVMArrayType2, LLVMConstGEP2, LLVMConstInt, LLVMConstString, LLVMInt32Type,
        LLVMInt8Type, LLVMSetGlobalConstant, LLVMSetInitializer, LLVMSetLinkage,
    },
    prelude::{LLVMModuleRef, LLVMValueRef},
    LLVMLinkage,
};
use std::ffi::CString;

pub unsafe fn build_literal_cstring(llvm_module: LLVMModuleRef, value: &CString) -> LLVMValueRef {
    let length = value.as_bytes_with_nul().len();
    let storage_type = LLVMArrayType2(LLVMInt8Type(), length.try_into().unwrap());

    let read_only = LLVMAddGlobal(llvm_module, storage_type, cstr!("").as_ptr());
    LLVMSetLinkage(read_only, LLVMLinkage::LLVMInternalLinkage);
    LLVMSetGlobalConstant(read_only, true as i32);
    LLVMSetInitializer(
        read_only,
        LLVMConstString(value.as_ptr(), length.try_into().unwrap(), true as i32),
    );

    let mut indicies = [
        LLVMConstInt(LLVMInt32Type(), 0, true as i32),
        LLVMConstInt(LLVMInt32Type(), 0, true as i32),
    ];

    LLVMConstGEP2(
        storage_type,
        read_only,
        indicies.as_mut_ptr(),
        indicies.len() as _,
    )
}
