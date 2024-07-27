use super::{abi_type::ABIType, arch::Arch, cxx::Itanium};
use crate::{
    data_units::ByteUnits,
    ir,
    llvm_backend::{ctx::BackendCtx, error::BackendError},
};
use llvm_sys::prelude::LLVMTypeRef;

#[derive(Clone, Debug)]
pub struct InAllocaStruct {
    pub ty: LLVMTypeRef,
    pub alignment: ByteUnits,
}

#[derive(Clone, Debug)]
pub struct ABIFunction {
    pub parameter_types: Vec<ABIParam>,
    pub return_type: ABIParam,
    pub inalloca_combined_struct: Option<InAllocaStruct>,
}

#[derive(Clone, Debug)]
pub struct ABIParam {
    pub abi_type: ABIType,
    pub ir_type: ir::Type,
}

impl ABIFunction {
    pub fn new<'a>(
        ctx: &BackendCtx,
        parameter_types: impl Iterator<Item = &'a ir::Type>,
        return_type: &ir::Type,
        is_variadic: bool,
    ) -> Result<Self, BackendError> {
        match &ctx.arch {
            Arch::X86_64(_abi) => todo!(),
            Arch::AARCH64(abi) => {
                let itanium = Itanium {
                    target_info: &ctx.ir_module.target_info,
                    type_layout_cache: &ctx.type_layout_cache,
                };

                abi.compute_info(ctx, itanium, parameter_types, return_type, is_variadic)
            }
        }
    }
}
