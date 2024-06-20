use super::{abi_type::ABIType, arch::Arch};
use llvm_sys::prelude::LLVMTypeRef;

#[derive(Clone)]
pub struct ABIFunction {
    pub parameter_types: Vec<ABIType>,
    pub return_type: ABIType,
}

impl ABIFunction {
    pub fn new(
        arch: Arch,
        parameter_types: &[LLVMTypeRef],
        return_type: Option<LLVMTypeRef>,
    ) -> Self {
        match arch {
            Arch::X86_64(abi) => abi.function(parameter_types, return_type),
            Arch::AARCH64(abi) => abi.function(parameter_types, return_type),
        }
    }
}
