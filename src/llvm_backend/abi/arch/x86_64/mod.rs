mod avx_level;
mod reg_count;
mod sysv;
mod win64;

use super::super::abi_function::ABIFunction;
use crate::{
    backend::BackendError,
    ir,
    llvm_backend::{abi::cxx::Itanium, ctx::BackendCtx},
};
pub use avx_level::AvxLevel;
use llvm_sys::LLVMCallConv;
pub use sysv::SysV;
pub use win64::Win64;

#[derive(Clone, Debug)]
pub enum X86_64 {
    SysV(SysV),
    Win64(Win64),
}

impl X86_64 {
    pub fn compute_info<'a>(
        &self,
        ctx: &BackendCtx,
        abi: Itanium,
        original_parameter_types: impl Iterator<Item = &'a ir::Type>,
        num_required: usize,
        original_return_type: &ir::Type,
        calling_convention: LLVMCallConv,
    ) -> Result<ABIFunction, BackendError> {
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
