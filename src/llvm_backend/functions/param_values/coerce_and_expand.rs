use super::{helpers::build_mem_tmp_with_alignment, ParamValues};
use crate::{
    ir,
    llvm_backend::{
        abi::abi_type::{get_struct_field_types, is_padding_for_coerce_expand, is_struct_type},
        address::Address,
        builder::Builder,
        ctx::{BackendCtx, FunctionSkeleton},
        error::BackendError,
        functions::{param_values::value::ParamValue, params_mapping::ParamRange},
    },
};
use cstr::cstr;
use llvm_sys::{
    core::LLVMGetParam,
    prelude::{LLVMTypeRef, LLVMValueRef},
};

impl ParamValues {
    pub fn push_coerce_and_expand(
        &mut self,
        builder: &Builder,
        ctx: &BackendCtx,
        skeleton: &FunctionSkeleton,
        param_range: ParamRange,
        ir_param_type: &ir::Type,
        alloca_point: LLVMValueRef,
        coerce_to_type: LLVMTypeRef,
    ) -> Result<(), BackendError> {
        assert!(is_struct_type(coerce_to_type));

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
        let field_types = get_struct_field_types(coerce_to_type);

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
