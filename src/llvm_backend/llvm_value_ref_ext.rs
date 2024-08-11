use llvm_sys::{
    core::{LLVMConstInt, LLVMInt64Type},
    prelude::LLVMValueRef,
};

pub trait LLVMValueRefExt: Sized + Copy {
    fn new_u64(value: u64) -> Self;
}

impl LLVMValueRefExt for LLVMValueRef {
    fn new_u64(value: u64) -> Self {
        unsafe { LLVMConstInt(LLVMInt64Type(), value, false as _) }
    }
}
