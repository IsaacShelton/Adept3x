use super::param_value::ParamValue;
use crate::{
    ir,
    llvm_backend::{
        abi::abi_type::InAlloca,
        address::Address,
        backend_type::to_backend_mem_type,
        builder::{build_load, build_struct_gep, Builder},
        ctx::BackendCtx,
        error::BackendError,
        raw_address::RawAddress,
    },
    target_info::type_layout::TypeLayoutCache,
};
use llvm_sys::prelude::LLVMTypeRef;
use std::ops::Range;

pub struct ParamValues {
    values: Vec<ParamValue>,
}

impl ParamValues {
    pub fn new() -> Self {
        Self {
            values: Vec::<ParamValue>::with_capacity(16),
        }
    }

    pub fn push_inalloca(
        &mut self,
        builder: &Builder,
        ctx: &BackendCtx,
        inalloca: &InAlloca,
        llvm_param_range: Range<usize>,
        arg_struct: &Address,
        ty: &ir::Type,
        type_layout_cache: &TypeLayoutCache,
        inalloca_subtypes: &[LLVMTypeRef],
    ) -> Result<(), BackendError> {
        assert_eq!(llvm_param_range.count(), 0);

        let field_index = inalloca.alloca_field_index;

        let mut value = build_struct_gep(
            builder,
            ctx.target_data,
            arg_struct,
            field_index as usize,
            Some(inalloca_subtypes),
        );

        if inalloca.indirect {
            value = Address::from(RawAddress {
                base: build_load(builder, value),
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
