use llvm_sys::{
    core::{
        LLVMAddAttributeAtIndex, LLVMCreateEnumAttribute, LLVMGetEnumAttributeKindForName,
        LLVMGetGlobalContext,
    },
    prelude::{LLVMAttributeRef, LLVMValueRef},
    LLVMAttributeFunctionIndex,
};
use std::ffi::CStr;

pub fn create_enum_attribute(name: &CStr, value: u64) -> LLVMAttributeRef {
    unsafe {
        LLVMCreateEnumAttribute(
            LLVMGetGlobalContext(),
            LLVMGetEnumAttributeKindForName(name.as_ptr(), name.count_bytes()),
            value,
        )
    }
}

pub fn add_param_attribute(
    function: LLVMValueRef,
    param_index: usize,
    attribute: LLVMAttributeRef,
) {
    unsafe { LLVMAddAttributeAtIndex(function, 1 + u32::try_from(param_index).unwrap(), attribute) }
}

pub fn add_func_attribute(function: LLVMValueRef, attribute: LLVMAttributeRef) {
    unsafe { LLVMAddAttributeAtIndex(function, LLVMAttributeFunctionIndex, attribute) }
}
