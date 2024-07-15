use super::ParamValues;
use crate::{
    data_units::ByteUnits,
    ir,
    llvm_backend::{
        abi::has_scalar_evaluation_kind,
        builder::Builder,
        ctx::{BackendCtx, FunctionSkeleton},
        error::BackendError,
        functions::{
            param_values::{
                helpers::{build_mem_tmp, emit_load_of_scalar},
                value::ParamValue,
            },
            params_mapping::ParamRange,
            prologue::helpers::make_natural_address_for_pointer,
        },
    },
};
use cstr::cstr;
use llvm_sys::{
    core::{LLVMBuildMemCpy, LLVMConstInt, LLVMGetParam},
    prelude::LLVMValueRef,
    target::LLVMIntPtrType,
};

impl ParamValues {
    #[allow(clippy::too_many_arguments)]
    pub fn push_indirect(
        &mut self,
        builder: &Builder,
        ctx: &BackendCtx,
        skeleton: &FunctionSkeleton,
        param_range: ParamRange,
        ty: &ir::Type,
        indirect_alignment: ByteUnits,
        realign: bool,
        aliased: bool,
        alloca_point: LLVMValueRef,
    ) -> Result<(), BackendError> {
        assert_eq!(param_range.len(), 1);

        let index = param_range.start.try_into().unwrap();
        let argument = unsafe { LLVMGetParam(skeleton.function, index) };

        let mut param_address =
            make_natural_address_for_pointer(ctx, argument, ty, Some(indirect_alignment), false)?;

        if has_scalar_evaluation_kind(ty) {
            let value = emit_load_of_scalar(builder, &param_address, false, ty);
            self.values.push(ParamValue::Direct(value));
            return Ok(());
        }

        if realign || aliased {
            let aligned_tmp = build_mem_tmp(ctx, builder, alloca_point, ty, cstr!("coerce"))?;
            let size = ctx.type_layout_cache.get(ty).width;

            let pointer_sized_int_ty = unsafe { LLVMIntPtrType(ctx.target_data.get()) };

            let num_bytes =
                unsafe { LLVMConstInt(pointer_sized_int_ty, size.bytes(), false as i32) };

            unsafe {
                LLVMBuildMemCpy(
                    builder.get(),
                    aligned_tmp.base_pointer(),
                    aligned_tmp.alignment.bytes().try_into().unwrap(),
                    param_address.base_pointer(),
                    param_address.base.alignment.bytes().try_into().unwrap(),
                    num_bytes,
                );
            }

            param_address = aligned_tmp.into();
        }

        self.values.push(ParamValue::Indirect(param_address));
        Ok(())
    }
}
