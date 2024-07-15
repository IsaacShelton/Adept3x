use super::ParamValues;
use crate::{
    ir,
    llvm_backend::{
        abi::abi_type::InAlloca,
        address::Address,
        backend_type::to_backend_mem_type,
        builder::{build_load, build_struct_gep, Builder},
        ctx::BackendCtx,
        error::BackendError,
        functions::{param_values::value::ParamValue, params_mapping::ParamRange},
        raw_address::RawAddress,
    },
    target_info::type_layout::TypeLayoutCache,
};
use llvm_sys::prelude::LLVMTypeRef;

impl ParamValues {
    #[allow(clippy::too_many_arguments)]
    pub fn push_inalloca(
        &mut self,
        builder: &Builder,
        ctx: &BackendCtx,
        inalloca: &InAlloca,
        param_range: ParamRange,
        arg_struct: &Address,
        ty: &ir::Type,
        type_layout_cache: &TypeLayoutCache,
        inalloca_subtypes: &[LLVMTypeRef],
    ) -> Result<(), BackendError> {
        assert_eq!(param_range.len(), 0);

        let mut value = build_struct_gep(
            builder,
            ctx.target_data,
            arg_struct,
            inalloca.alloca_field_index as usize,
            Some(inalloca_subtypes),
        );

        if inalloca.indirect {
            value = Address::from(RawAddress {
                base: build_load(builder, &value, false),
                nullable: false,
                alignment: type_layout_cache.get(ty).alignment,
                element_type: unsafe {
                    to_backend_mem_type(ctx.for_making_type(), type_layout_cache, ty, false)?
                },
            });
        }

        self.values.push(ParamValue::Indirect(value));
        Ok(())
    }
}
