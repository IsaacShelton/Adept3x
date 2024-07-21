use super::ParamValues;
use crate::{
    ir,
    llvm_backend::{
        abi::abi_type::InAlloca,
        address::Address,
        backend_type::to_backend_mem_type,
        builder::{Builder, Volatility},
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
        ir_param_type: &ir::Type,
        type_layout_cache: &TypeLayoutCache,
        inalloca_subtypes: &[LLVMTypeRef],
    ) -> Result<(), BackendError> {
        assert_eq!(param_range.len(), 0);

        let mut value = builder.gep_struct(
            ctx.target_data,
            arg_struct,
            inalloca.alloca_field_index as usize,
            Some(inalloca_subtypes),
        );

        if inalloca.indirect {
            value = Address::from(RawAddress {
                base: builder.load(&value, Volatility::Normal),
                nullable: false,
                alignment: type_layout_cache.get(ir_param_type).alignment,
                element_type: unsafe {
                    to_backend_mem_type(
                        ctx.for_making_type(),
                        type_layout_cache,
                        ir_param_type,
                        false,
                    )?
                },
            });
        }

        self.values.push(ParamValue::Indirect(value));
        Ok(())
    }
}
