use llvm_sys::{
    LLVMLinkage,
    core::{
        LLVMAddGlobal, LLVMArrayType2, LLVMConstGEP2, LLVMConstInt, LLVMConstString, LLVMInt8Type,
        LLVMInt32Type, LLVMSetGlobalConstant, LLVMSetInitializer, LLVMSetLinkage,
    },
    prelude::{LLVMModuleRef, LLVMValueRef},
};
use std::ffi::CStr;

pub unsafe fn build_literal_cstring(llvm_module: LLVMModuleRef, value: &CStr) -> LLVMValueRef {
    let length = value.to_bytes_with_nul().len();
    let storage_type = LLVMArrayType2(LLVMInt8Type(), length.try_into().unwrap());

    let read_only = LLVMAddGlobal(llvm_module, storage_type, c"".as_ptr());
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
