use super::Builder;
use crate::{data_units::ByteUnits, llvm_backend::address::Address};
use cstr::cstr;
use llvm_sys::{
    core::{LLVMBuildLoad2, LLVMSetAlignment, LLVMSetVolatile},
    prelude::{LLVMTypeRef, LLVMValueRef},
};
use std::ffi::CStr;

pub fn build_load(builder: &Builder, address: &Address, is_volitile: bool) -> LLVMValueRef {
    build_aligned_load(
        builder,
        address.element_type(),
        address.base_pointer(),
        address.base.alignment,
        is_volitile,
        cstr!(""),
    )
}

pub fn build_aligned_load(
    builder: &Builder,
    ty: LLVMTypeRef,
    pointer: LLVMValueRef,
    alignment: ByteUnits,
    is_volitile: bool,
    name: &CStr,
) -> LLVMValueRef {
    unsafe {
        let load = LLVMBuildLoad2(builder.get(), ty, pointer, name.as_ptr());
        LLVMSetAlignment(load, alignment.bytes().try_into().unwrap());
        LLVMSetVolatile(load, i32::from(is_volitile));
        load
    }
}
