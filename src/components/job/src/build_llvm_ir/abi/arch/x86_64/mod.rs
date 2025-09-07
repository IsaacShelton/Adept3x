mod avx_level;
mod reg_count;
mod sysv;
mod win64;

use super::super::abi_function::ABIFunction;
use crate::{
    build_llvm_ir::{abi::cxx::Itanium, ctx::BackendCtx},
    ir,
};
pub use avx_level::AvxLevel;
use diagnostics::ErrorDiagnostic;
use llvm_sys::LLVMCallConv;
pub use sysv::{SysV, SysVOs};
pub use win64::Win64;

#[derive(Clone, Debug)]
pub enum X86_64 {
    SysV(SysV),
    Win64(Win64),
}

impl X86_64 {
    pub fn compute_info<'env>(
        &self,
        ctx: &BackendCtx<'_, 'env>,
        abi: Itanium<'_, 'env>,
        original_parameter_types: impl Iterator<Item = &'env ir::Type<'env>>,
        num_required: usize,
        original_return_type: &'env ir::Type<'env>,
        calling_convention: LLVMCallConv,
    ) -> Result<ABIFunction<'env>, ErrorDiagnostic> {
        match self {
            Self::SysV(sysv) => sysv.compute_info(
                ctx,
                &abi,
                original_parameter_types,
                num_required,
                original_return_type,
                calling_convention,
            ),
            Self::Win64(win64) => win64.compute_info(
                ctx,
                &abi,
                original_parameter_types,
                original_return_type,
                calling_convention,
            ),
        }
    }
}
