use super::{ParamValueConstructionCtx, ParamValues};
use crate::{
    ir,
    llvm_backend::{
        abi::abi_type::kinds::{get_type_expansion, TypeExpansion},
        address::Address,
        backend_type::{to_backend_mem_type, to_backend_type},
        builder::Builder,
        ctx::BackendCtx,
        error::BackendError,
        functions::helpers::build_mem_tmp_with_alignment,
        llvm_type_ref_ext::LLVMTypeRefExt,
    },
};
use cstr::cstr;
use llvm_sys::{core::LLVMGetParam, prelude::LLVMValueRef};

impl ParamValues {
    pub fn push_expand(
        &mut self,
        construction_context: ParamValueConstructionCtx,
    ) -> Result<(), BackendError> {
        // NOTE: We assume no bitfields are allowed to be expanded.
        // This should have already been checked when creating the ABIType.
        // We also assume no vector types are allowed here, as we don't support them yet

        let ParamValueConstructionCtx {
            builder,
            ctx,
            skeleton,
            param_range,
            ir_param_type,
            alloca_point,
        } = construction_context;

        let llvm_function = skeleton.function;

        assert!(
            !ir_param_type.is_vector(),
            "ParamValues::push_expand does not support vector types yet"
        );

        let user_specified_alignment = ctx.type_layout_cache.get(ir_param_type).alignment;

        let alloca = build_mem_tmp_with_alignment(
            ctx,
            builder,
            alloca_point,
            ir_param_type,
            user_specified_alignment,
            cstr!(""),
        )?;

        let mut param_range_iter = param_range.iter();

        expand_type_from_args(
            ctx,
            builder,
            llvm_function,
            ir_param_type,
            &Address::from(alloca),
            &mut param_range_iter,
        )?;

        assert!(
            param_range_iter.next().is_none(),
            "all subtypes should have been un-expanded when receiving type with ABI expand pass mode"
        );

        Ok(())
    }
}

fn expand_type_from_args(
    ctx: &BackendCtx,
    builder: &Builder,
    llvm_function: LLVMValueRef,
    ir_type: &ir::Type,
    base_address: &Address,
    param_range_iter: &mut impl Iterator<Item = usize>,
) -> Result<(), BackendError> {
    let expansion = get_type_expansion(ir_type, &ctx.type_layout_cache, ctx.ir_module);

    match expansion {
        TypeExpansion::FixedArray(fixed_array) => {
            for item_i in 0..fixed_array.length {
                let element_address = builder.gep(ctx.target_data, base_address, 0, item_i);

                expand_type_from_args(
                    ctx,
                    builder,
                    llvm_function,
                    &fixed_array.inner,
                    &element_address,
                    param_range_iter,
                )?;
            }
        }
        TypeExpansion::Record(fields) => {
            // NOTE: We don't support C++ inheritance here

            let llvm_type = unsafe { to_backend_type(ctx.for_making_type(), ir_type)? };
            let precomputed_field_types = llvm_type.field_types();
            assert_eq!(fields.len(), precomputed_field_types.len());

            for (field_i, field) in fields.iter().enumerate() {
                // NOTE: This calculation of storage address should be right as long as we aren't
                // using any bitfields
                let storage_address = builder.gep_struct(
                    ctx.target_data,
                    base_address,
                    field_i,
                    Some(precomputed_field_types.as_slice()),
                );

                assert!(!field.is_bitfield() && !field.ir_type.is_vector());

                let storage_type = unsafe {
                    to_backend_mem_type(
                        ctx.for_making_type(),
                        &ctx.type_layout_cache,
                        &field.ir_type,
                        field.is_bitfield(),
                    )?
                };

                let storage_address = storage_address.with_element_type(storage_type);

                expand_type_from_args(
                    ctx,
                    builder,
                    llvm_function,
                    &field.ir_type,
                    &storage_address,
                    param_range_iter,
                )?;
            }
        }
        TypeExpansion::Complex(_) => {
            todo!("un-expanding complex values passed via expand ABI pass mode is not support yet")
        }
        TypeExpansion::None => {
            let argument = unsafe {
                LLVMGetParam(
                    llvm_function,
                    param_range_iter
                        .next()
                        .expect("argument value for destination being un-expanded into")
                        .try_into()
                        .unwrap(),
                )
            };

            assert!(
                !ir_type.is_vector(),
                "un-expanding vector values passed via expand ABI pass mode is not supported yet"
            );

            builder.store(argument, base_address);
        }
    }

    Ok(())
}
