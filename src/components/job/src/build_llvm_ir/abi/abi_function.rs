use super::{abi_type::ABIType, arch::Arch, cxx::Itanium};
use crate::{
    build_llvm_ir::{backend_type::to_backend_type, ctx::BackendCtx},
    ir,
};
use data_units::ByteUnits;
use diagnostics::ErrorDiagnostic;
use llvm_sys::{LLVMCallConv, prelude::LLVMTypeRef};

#[derive(Clone, Debug)]
pub struct InAllocaStruct {
    pub ty: LLVMTypeRef,
    pub alignment: ByteUnits,
}

#[derive(Clone, Debug)]
pub struct ABIFunction<'env> {
    pub parameter_types: Vec<ABIParam<'env>>,
    pub return_type: ABIParam<'env>,
    pub inalloca_combined_struct: Option<InAllocaStruct>,
    pub head_max_vector_width: ByteUnits,
}

#[derive(Clone, Debug)]
pub struct ABIParam<'env> {
    pub abi_type: ABIType,
    pub ir_type: &'env ir::Type<'env>,
}

impl<'env> ABIFunction<'env> {
    pub fn new(
        ctx: &BackendCtx<'_, 'env>,
        parameter_types: impl Iterator<Item = &'env ir::Type<'env>>,
        num_required: usize,
        return_type: &'env ir::Type<'env>,
        is_variadic: bool,
    ) -> Result<Self, ErrorDiagnostic> {
        let mut abi_function =
            Self::new_agnostic(ctx, parameter_types, num_required, return_type, is_variadic)?;

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

    pub fn new_agnostic(
        ctx: &BackendCtx<'_, 'env>,
        parameter_types: impl Iterator<Item = &'env ir::Type<'env>>,
        num_required: usize,
        return_type: &'env ir::Type,
        is_variadic: bool,
    ) -> Result<Self, ErrorDiagnostic> {
        let calling_convention = LLVMCallConv::LLVMCCallConv;
        let itanium = Itanium {
            target: &ctx.meta.target,
            type_layout_cache: &ctx.type_layout_cache,
        };

        match &ctx.arch {
            Arch::X86_64(abi) => abi.compute_info(
                ctx,
                itanium,
                parameter_types,
                num_required,
                return_type,
                calling_convention,
            ),
            Arch::Aarch64(abi) => {
                abi.compute_info(ctx, itanium, parameter_types, return_type, is_variadic)
            }
        }
    }
}
