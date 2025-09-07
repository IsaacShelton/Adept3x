use super::Builder;
use llvm_sys::{
    core::LLVMBuildIntCast2,
    prelude::{LLVMTypeRef, LLVMValueRef},
};

impl<'env> Builder<'env> {
    pub fn int_cast(
        &self,
        value: LLVMValueRef,
        integer_type: LLVMTypeRef,
        signed: bool,
    ) -> LLVMValueRef {
        unsafe { LLVMBuildIntCast2(self.get(), value, integer_type, signed as _, c"".as_ptr()) }
    }
}
