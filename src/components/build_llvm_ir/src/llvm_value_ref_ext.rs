use llvm_sys::{
    core::{LLVMConstInt, LLVMInt32Type, LLVMInt64Type},
    prelude::LLVMValueRef,
};

pub trait LLVMValueRefExt: Sized + Copy {
    fn new_i32(value: i32) -> Self;
    fn new_u64(value: u64) -> Self;
}

impl LLVMValueRefExt for LLVMValueRef {
    fn new_i32(value: i32) -> Self {
        unsafe { LLVMConstInt(LLVMInt32Type(), value as u64, true as _) }
    }

    fn new_u64(value: u64) -> Self {
        unsafe { LLVMConstInt(LLVMInt64Type(), value, false as _) }
    }
}
