use super::{abi_type::ABIType, arch::Arch, cxx::Itanium};
use crate::{
    ir,
    llvm_backend::{ctx::ToBackendTypeCtx, error::BackendError},
};
use llvm_sys::prelude::LLVMTypeRef;
use std::borrow::Borrow;

#[derive(Clone, Debug)]
pub struct ABIFunction {
    pub parameter_types: Vec<ABIParam>,
    pub return_type: ABIType,
    pub inalloca_combined_struct: Option<LLVMTypeRef>,
}

#[derive(Clone, Debug)]
pub struct ABIParam {
    pub abi_type: ABIType,
    pub ir_type: ir::Type,
}

impl ABIFunction {
    pub fn new<'a>(
        ctx: impl Borrow<ToBackendTypeCtx<'a>>,
        arch: Arch,
        parameter_types: &[ir::Type],
        return_type: &ir::Type,
        is_variadic: bool,
    ) -> Result<Self, BackendError> {
        let info = arch.core_info();

        match arch {
            Arch::X86_64(_abi) => todo!(),
            Arch::AARCH64(abi) => {
                let itanium = Itanium {
                    target_info: info.target_info,
                    type_layout_cache: info.type_layout_cache,
                };

                abi.compute_info(
                    ctx.borrow(),
                    itanium,
                    parameter_types,
                    return_type,
                    is_variadic,
                )
            }
        }
    }
}
