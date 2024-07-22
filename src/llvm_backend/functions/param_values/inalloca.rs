use super::{ParamValueConstructionCtx, ParamValues};
use crate::llvm_backend::{
    abi::abi_type::InAlloca, address::Address, backend_type::to_backend_mem_type,
    builder::Volatility, error::BackendError, functions::param_values::value::ParamValue,
    raw_address::RawAddress,
};
use llvm_sys::prelude::LLVMTypeRef;

impl ParamValues {
    #[allow(clippy::too_many_arguments)]
    pub fn push_inalloca(
        &mut self,
        construction_ctx: ParamValueConstructionCtx,
        inalloca: &InAlloca,
        arg_struct: &Address,
        inalloca_subtypes: &[LLVMTypeRef],
    ) -> Result<(), BackendError> {
        let ParamValueConstructionCtx {
            builder,
            ctx,
            param_range,
            ir_param_type,
            ..
        } = construction_ctx;

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
                alignment: ctx.type_layout_cache.get(ir_param_type).alignment,
                element_type: unsafe {
                    to_backend_mem_type(
                        ctx.for_making_type(),
                        &ctx.type_layout_cache,
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
