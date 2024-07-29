use super::{abi_type::ABIType, arch::Arch, cxx::Itanium};
use crate::{
    data_units::ByteUnits,
    ir,
    llvm_backend::{backend_type::to_backend_type, ctx::BackendCtx, error::BackendError},
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
        let mut abi_function = Self::new_agnostic(ctx, parameter_types, return_type, is_variadic)?;

        // Fill in default coerce type for return type
        abi_function
            .return_type
            .abi_type
            .coerce_to_type_if_missing(|| unsafe {
                to_backend_type(ctx.for_making_type(), &abi_function.return_type.ir_type)
            })?;

        // Fill in default coerce types for parameters
        for abi_param in abi_function.parameter_types.iter_mut() {
            abi_param.abi_type.coerce_to_type_if_missing(|| unsafe {
                to_backend_type(ctx.for_making_type(), &abi_param.ir_type)
            })?;
        }

        Ok(abi_function)
    }

    pub fn new_agnostic<'a>(
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
