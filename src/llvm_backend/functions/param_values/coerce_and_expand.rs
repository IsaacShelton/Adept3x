use super::{ParamValueConstructionCtx, ParamValues};
use crate::llvm_backend::{
    abi::abi_type::is_padding_for_coerce_expand,
    address::Address,
    error::BackendError,
    functions::{helpers::build_mem_tmp_with_alignment, param_values::value::ParamValue},
    llvm_type_ref_ext::LLVMTypeRefExt,
};
use cstr::cstr;
use llvm_sys::{core::LLVMGetParam, prelude::LLVMTypeRef};

impl ParamValues {
    pub fn push_coerce_and_expand(
        &mut self,
        construction_ctx: ParamValueConstructionCtx,
        coerce_to_type: LLVMTypeRef,
    ) -> Result<(), BackendError> {
        assert!(coerce_to_type.is_struct());

        let ParamValueConstructionCtx {
            builder,
            ctx,
            skeleton,
            param_range,
            ir_param_type,
            alloca_point,
        } = construction_ctx;

        let user_specified_alignment = ctx.type_layout_cache.get(ir_param_type).alignment;
        let alloca = Address::from(build_mem_tmp_with_alignment(
            ctx,
            builder,
            alloca_point,
            ir_param_type,
            user_specified_alignment,
            cstr!(""),
        )?);

        self.values.push(ParamValue::Indirect(alloca.clone()));

        let alloca = alloca.with_element_type(coerce_to_type);
        let field_types = coerce_to_type.field_types();

        for (field_i, llvm_param_i) in param_range.iter().enumerate() {
            if is_padding_for_coerce_expand(field_types[field_i]) {
                continue;
            }

            let element_address = builder.gep_struct(
                ctx.target_data,
                &alloca,
                field_i,
                Some(field_types.as_slice()),
            );

            let element =
                unsafe { LLVMGetParam(skeleton.function, llvm_param_i.try_into().unwrap()) };

            builder.store(element, &element_address);
        }

        Ok(())
    }
}
