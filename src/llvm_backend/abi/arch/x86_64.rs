use super::super::abi_function::ABIFunction;
use llvm_sys::prelude::LLVMTypeRef;

#[derive(Clone, Debug)]
pub struct X86_64 {
    pub is_windows: Variant,
}

#[derive(Clone, Debug)]
pub enum Variant {
    Normal,
    Win64,
}

impl X86_64 {
    pub fn function(
        &self,
        _parameter_types: &[LLVMTypeRef],
        _return_type: Option<LLVMTypeRef>,
    ) -> ABIFunction {
        todo!("X86_64 function")
    }
}
