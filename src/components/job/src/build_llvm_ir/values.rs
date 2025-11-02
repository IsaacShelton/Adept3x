use super::{
    backend_type::to_backend_type, builder::Builder, ctx::BackendCtx,
    null_terminated_string::build_literal_cstring, value_catalog::ValueCatalog,
};
use crate::ir;
use diagnostics::ErrorDiagnostic;
use llvm_sys::{
    core::*,
    prelude::{LLVMBool, LLVMValueRef},
};
use std::ffi::{c_double, c_ulonglong};

pub unsafe fn build_value<'env>(
    ctx: &BackendCtx<'_, 'env>,
    value_catalog: &ValueCatalog,
    _builder: &Builder<'env>,
    value: &ir::Value<'env>,
) -> Result<LLVMValueRef, ErrorDiagnostic> {
    Ok(match value {
        ir::Value::Literal(literal) => match literal {
            ir::Literal::Boolean(value) => {
                LLVMConstInt(LLVMInt1Type(), *value as c_ulonglong, false as LLVMBool)
            }
            ir::Literal::Integer(immediate) => LLVMConstInt(
                LLVMIntType(immediate.bits().bytes().to_bits().bits() as u32),
                immediate.value().raw_data() as c_ulonglong,
                immediate.value().sign().is_signed() as LLVMBool,
            ),
            ir::Literal::Float32(value) => LLVMConstReal(LLVMFloatType(), *value as c_double),
            ir::Literal::Float64(value) => LLVMConstReal(LLVMDoubleType(), *value as c_double),
            ir::Literal::NullTerminatedString(value) => {
                build_literal_cstring(ctx.backend_module.get(), value)
            }
            ir::Literal::Void => LLVMGetUndef(LLVMVoidType()),
            ir::Literal::Zeroed(ir_type) => {
                let backend_type = to_backend_type(&ctx.for_making_type(), ir_type)?;
                LLVMConstNull(backend_type)
            }
        },
        ir::Value::Reference(reference) => value_catalog
            .get(reference)
            .expect("referenced value exists"),
    })
}
