use crate::llvm_backend::abi::abi_function::ABIFunction;
use llvm_sys::prelude::LLVMTypeRef;

#[derive(Clone, Debug)]
pub struct AARCH64 {
    pub variant: Variant,
}

#[derive(Clone, Debug)]
pub enum Variant {
    DarwinPCS,
    Win64,
    AAPCS,
}

impl AARCH64 {
    pub fn function(
        &self,
        _original_parameter_types: &[LLVMTypeRef],
        _original_return_type: Option<LLVMTypeRef>,
    ) -> ABIFunction {
        todo!("AARCH64 function");
    }
}
