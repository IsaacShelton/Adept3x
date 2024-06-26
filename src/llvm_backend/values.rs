use super::{
    backend_type::to_backend_type, builder::Builder, ctx::BackendCtx, error::BackendError,
    null_terminated_string::build_literal_cstring, value_catalog::ValueCatalog,
};
use crate::ir;
use llvm_sys::{
    core::*,
    prelude::{LLVMBool, LLVMValueRef},
};
use std::{
    collections::HashSet,
    ffi::{c_double, c_ulonglong},
};

pub unsafe fn build_value(
    ctx: &BackendCtx<'_>,
    value_catalog: &ValueCatalog,
    _builder: &Builder,
    value: &ir::Value,
) -> Result<LLVMValueRef, BackendError> {
    Ok(match value {
        ir::Value::Literal(literal) => match literal {
            ir::Literal::Boolean(value) => {
                LLVMConstInt(LLVMInt1Type(), *value as c_ulonglong, false as LLVMBool)
            }
            ir::Literal::Signed8(value) => {
                LLVMConstInt(LLVMInt8Type(), *value as c_ulonglong, true as LLVMBool)
            }
            ir::Literal::Signed16(value) => {
                LLVMConstInt(LLVMInt16Type(), *value as c_ulonglong, true as LLVMBool)
            }
            ir::Literal::Signed32(value) => {
                LLVMConstInt(LLVMInt32Type(), *value as c_ulonglong, true as LLVMBool)
            }
            ir::Literal::Signed64(value) => {
                LLVMConstInt(LLVMInt64Type(), *value as c_ulonglong, true as LLVMBool)
            }
            ir::Literal::Unsigned8(value) => {
                LLVMConstInt(LLVMInt8Type(), *value as c_ulonglong, false as LLVMBool)
            }
            ir::Literal::Unsigned16(value) => {
                LLVMConstInt(LLVMInt16Type(), *value as c_ulonglong, false as LLVMBool)
            }
            ir::Literal::Unsigned32(value) => {
                LLVMConstInt(LLVMInt32Type(), *value as c_ulonglong, false as LLVMBool)
            }
            ir::Literal::Unsigned64(value) => {
                LLVMConstInt(LLVMInt64Type(), *value as c_ulonglong, false as LLVMBool)
            }
            ir::Literal::Float32(value) => LLVMConstReal(LLVMFloatType(), *value as c_double),
            ir::Literal::Float64(value) => LLVMConstReal(LLVMDoubleType(), *value as c_double),
            ir::Literal::NullTerminatedString(value) => {
                build_literal_cstring(ctx.backend_module.get(), value)
            }
            ir::Literal::Void => LLVMGetUndef(LLVMVoidType()),
            ir::Literal::Zeroed(ir_type) => {
                let backend_type = to_backend_type(ctx, ir_type, &mut HashSet::new())?;
                LLVMConstNull(backend_type)
            }
        },
        ir::Value::Reference(reference) => value_catalog
            .get(reference)
            .expect("referenced value exists"),
    })
}
