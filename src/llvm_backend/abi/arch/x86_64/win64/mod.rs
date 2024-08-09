use crate::{
    ir,
    llvm_backend::{
        abi::{abi_function::ABIFunction, cxx::Itanium},
        ctx::BackendCtx,
        error::BackendError,
    },
};
use llvm_sys::LLVMCallConv;

#[derive(Clone, Debug)]
pub struct Win64 {}

impl Win64 {
    pub fn compute_info<'a>(
        &self,
        _ctx: &BackendCtx,
        _abi: &Itanium,
        _original_parameter_types: impl Iterator<Item = &'a ir::Type>,
        _num_required: usize,
        _original_return_type: &ir::Type,
        _is_variadic: bool,
        _calling_convention: LLVMCallConv,
    ) -> Result<ABIFunction, BackendError> {
        todo!("Win64::compute_info not supported yet")
    }
}
