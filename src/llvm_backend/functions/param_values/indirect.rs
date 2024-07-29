use super::{ParamValueConstructionCtx, ParamValues};
use crate::{
    data_units::ByteUnits,
    llvm_backend::{
        abi::has_scalar_evaluation_kind,
        builder::Volatility,
        error::BackendError,
        functions::{
            helpers::{build_mem_tmp, emit_load_of_scalar, make_natural_address_for_pointer},
            param_values::value::ParamValue,
        },
    },
};
use cstr::cstr;
use llvm_sys::core::{LLVMBuildMemCpy, LLVMConstInt, LLVMGetParam};

impl ParamValues {
    pub fn push_indirect(
        &mut self,
        construction_ctx: ParamValueConstructionCtx,
        indirect_alignment: ByteUnits,
        realign: bool,
        aliased: bool,
    ) -> Result<(), BackendError> {
        let ParamValueConstructionCtx {
            builder,
            ctx,
            skeleton,
            param_range,
            ir_param_type,
            alloca_point,
        } = construction_ctx;

        assert_eq!(param_range.len(), 1);

        let index = param_range.start.try_into().unwrap();
        let raw_argument_value = unsafe { LLVMGetParam(skeleton.function, index) };

        let mut indirect_pointer = make_natural_address_for_pointer(
            ctx,
            raw_argument_value,
            ir_param_type,
            Some(indirect_alignment),
            false,
        )?;

        if has_scalar_evaluation_kind(ir_param_type) {
            let value = emit_load_of_scalar(
                builder,
                &indirect_pointer,
                Volatility::Normal,
                ir_param_type,
            );
            self.values.push(ParamValue::Direct(value));
            return Ok(());
        }

        if realign || aliased {
            let aligned_on_stack =
                build_mem_tmp(ctx, builder, alloca_point, ir_param_type, cstr!("coerce"))?;

            let parameter_type_size = ctx.type_layout_cache.get(ir_param_type).width;

            let num_bytes = unsafe {
                LLVMConstInt(
                    ctx.target_data.pointer_sized_int_type(),
                    parameter_type_size.bytes(),
                    false as i32,
                )
            };

            unsafe {
                LLVMBuildMemCpy(
                    builder.get(),
                    aligned_on_stack.base_pointer(),
                    aligned_on_stack.alignment.bytes().try_into().unwrap(),
                    indirect_pointer.base_pointer(),
                    indirect_pointer.base.alignment.bytes().try_into().unwrap(),
                    num_bytes,
                );
            }

            indirect_pointer = aligned_on_stack.into();
        }

        self.values.push(ParamValue::Indirect(indirect_pointer));
        Ok(())
    }
}
