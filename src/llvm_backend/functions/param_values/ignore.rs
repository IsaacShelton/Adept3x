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
        ty: &ir::Type,
        alloca_point: LLVMValueRef,
    ) -> Result<(), BackendError> {
        assert_eq!(param_range.len(), 0);

        if has_scalar_evaluation_kind(ty) {
            let scalar_ty = unsafe { to_backend_type(ctx.for_making_type(), ty)? };
            let undef = unsafe { LLVMGetUndef(scalar_ty) };
            self.values.push(ParamValue::Direct(undef));
        } else {
            let tmp = build_mem_tmp(ctx, builder, alloca_point, ty, cstr!("tmp"))?;
            self.values.push(ParamValue::Indirect(tmp.into()));
        }

        Ok(())
    }
}
