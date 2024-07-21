use super::ParamValues;
use crate::{
    ir,
    llvm_backend::{
        abi::has_scalar_evaluation_kind,
        backend_type::to_backend_type,
        builder::Builder,
        ctx::BackendCtx,
        error::BackendError,
        functions::{
            param_values::{helpers::build_mem_tmp, value::ParamValue},
            params_mapping::ParamRange,
        },
    },
};
use cstr::cstr;
use llvm_sys::{core::LLVMGetUndef, prelude::LLVMValueRef};

impl ParamValues {
    pub fn push_ignore(
        &mut self,
        builder: &Builder,
        ctx: &BackendCtx,
        param_range: ParamRange,
        ir_param_type: &ir::Type,
        alloca_point: LLVMValueRef,
    ) -> Result<(), BackendError> {
        assert_eq!(param_range.len(), 0);

        if has_scalar_evaluation_kind(ir_param_type) {
            let scalar_ty = unsafe { to_backend_type(ctx.for_making_type(), ir_param_type)? };
            let undef = unsafe { LLVMGetUndef(scalar_ty) };
            self.values.push(ParamValue::Direct(undef));
        } else {
            let tmp = build_mem_tmp(ctx, builder, alloca_point, ir_param_type, cstr!("tmp"))?;
            self.values.push(ParamValue::Indirect(tmp.into()));
        }

        Ok(())
    }
}
