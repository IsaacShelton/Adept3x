use super::{ParamValueConstructionCtx, ParamValues};
use crate::{
    abi::{abi_function::ABIParam, has_scalar_evaluation_kind},
    address::Address,
    backend_type::to_backend_type,
    builder::{Builder, Volatility},
    functions::{
        helpers::{
            build_coerced_store, build_mem_tmp_with_alignment, build_tmp_alloca_address,
            emit_address_at_offset, emit_load_of_scalar,
        },
        param_values::value::ParamValue,
        params_mapping::ParamRange,
    },
    llvm_type_ref_ext::LLVMTypeRefExt,
    llvm_value_ref_ext::LLVMValueRefExt,
};
use backend::BackendError;
use cstr::cstr;
use llvm_sys::{
    core::{LLVMGetParam, LLVMTypeOf},
    prelude::{LLVMTypeRef, LLVMValueRef},
};

impl ParamValues {
    #[allow(clippy::too_many_arguments)]
    pub fn push_direct_or_extend(
        &mut self,
        construction_ctx: ParamValueConstructionCtx,
        abi_param: &ABIParam,
    ) -> Result<(), BackendError> {
        assert!(abi_param.abi_type.kind.is_direct() || abi_param.abi_type.kind.is_extend());

        let ParamValueConstructionCtx {
            builder,
            ctx,
            skeleton,
            param_range,
            ir_param_type,
            alloca_point,
        } = construction_ctx;

        let argument =
            unsafe { LLVMGetParam(skeleton.function, param_range.start.try_into().unwrap()) };

        let desired_llvm_param_type =
            unsafe { to_backend_type(ctx.for_making_type(), ir_param_type)? };

        let coerce_to_type = abi_param.abi_type.coerce_to_type().flatten().unwrap();
        let offset_align = abi_param.abi_type.get_direct_offset_align().unwrap();
        let is_struct = coerce_to_type.is_struct();

        apply_attributes(
            abi_param,
            desired_llvm_param_type,
            ir_param_type,
            param_range,
        );

        // Trivial argument value
        if !is_struct
            && coerce_to_type == unsafe { to_backend_type(ctx.for_making_type(), ir_param_type)? }
            && offset_align.offset.is_zero()
        {
            return self.push_direct_trivial(
                builder,
                coerce_to_type,
                argument,
                param_range,
                desired_llvm_param_type,
            );
        }

        if ir_param_type.is_fixed_vector() {
            todo!("direct/extend ABI pass mode for fixed vector types are not supported yet");
        }

        let user_specified_alignment = ctx.type_layout_cache.get(ir_param_type).alignment;

        let alloca = Address::from(build_mem_tmp_with_alignment(
            ctx,
            builder,
            alloca_point,
            ir_param_type,
            user_specified_alignment,
            cstr!(""),
        )?);

        let pointer =
            emit_address_at_offset(builder, ctx.target_data, &abi_param.abi_type, &alloca);

        // Flatten struct type if possible for better optimizations
        if abi_param.abi_type.can_be_flattened() == Some(true)
            && is_struct
            && coerce_to_type.num_fields() > 1
        {
            let struct_size = ctx.target_data.abi_size_of_type(coerce_to_type);
            let pointer_element_size = ctx.target_data.abi_size_of_type(pointer.element_type());

            // NOTE: We don't support scalable SIMD vector types
            let source_size = struct_size;
            let destination_size = pointer_element_size;

            let address_to_store_into = if source_size < destination_size {
                pointer.with_element_type(coerce_to_type)
            } else {
                build_tmp_alloca_address(
                    builder,
                    alloca_point,
                    coerce_to_type,
                    alloca.base.alignment,
                    cstr!("coerce"),
                    None,
                )
                .into()
            };

            let elements = coerce_to_type.field_types();
            assert_eq!(elements.len(), param_range.len());

            for (field_i, llvm_parameter_i) in param_range.iter().enumerate() {
                let argument = unsafe {
                    LLVMGetParam(skeleton.function, llvm_parameter_i.try_into().unwrap())
                };

                let element_pointer = builder.gep_struct(
                    ctx.target_data,
                    &address_to_store_into,
                    field_i,
                    Some(elements.as_slice()),
                );

                builder.store(argument, &element_pointer);
            }

            if source_size > destination_size {
                builder.memcpy(
                    &pointer,
                    &address_to_store_into,
                    LLVMValueRef::new_u64(destination_size.bytes()),
                );
            }
        } else {
            assert_eq!(param_range.len(), 1);

            let argument =
                unsafe { LLVMGetParam(skeleton.function, param_range.start.try_into().unwrap()) };
            build_coerced_store(builder, ctx.target_data, argument, &pointer, alloca_point);
        }

        self.values
            .push(if has_scalar_evaluation_kind(ir_param_type) {
                ParamValue::Direct(emit_load_of_scalar(
                    builder,
                    &alloca,
                    Volatility::Normal,
                    ir_param_type,
                ))
            } else {
                ParamValue::Indirect(alloca)
            });

        Ok(())
    }

    fn push_direct_trivial(
        &mut self,
        builder: &Builder,
        coerce_to_type: LLVMTypeRef,
        argument: LLVMValueRef,
        param_range: ParamRange,
        desired_llvm_param_type: LLVMTypeRef,
    ) -> Result<(), BackendError> {
        assert_eq!(param_range.len(), 1);
        let mut value = argument;

        // Ensure argument is correct type
        if unsafe { LLVMTypeOf(value) } != coerce_to_type {
            value = builder.bitcast(value, coerce_to_type);
        }

        if unsafe { LLVMTypeOf(value) } != desired_llvm_param_type {
            value = builder.bitcast(value, desired_llvm_param_type);
        }

        self.values.push(ParamValue::Direct(value));
        Ok(())
    }
}

fn apply_attributes(
    abi_param: &ABIParam,
    desired_llvm_param_type: LLVMTypeRef,
    _ir_param_type: &ir::Type,
    param_range: ParamRange,
) {
    let coerce_to_type = abi_param.abi_type.coerce_to_type().flatten().unwrap();
    let offset_align = abi_param.abi_type.get_direct_offset_align().unwrap();

    if offset_align.offset.is_zero()
        && desired_llvm_param_type.is_pointer()
        && coerce_to_type.is_pointer()
    {
        assert_eq!(param_range.len(), 1);
        eprintln!("warning: apply_attributes for function prologues does not apply attributes yet");

        // TODO: Apply attributes
        // TODO: Apply restrict?
    }
}
