use crate::{
    data_units::ByteUnits,
    ir,
    llvm_backend::{
        address::Address,
        backend_type::to_backend_type,
        builder::{Builder, Volatility},
        ctx::BackendCtx,
        error::BackendError,
        functions::helpers::build_tmp_alloca_address,
        raw_address::RawAddress,
    },
};
use cstr::cstr;
use llvm_sys::{
    core::{LLVMBuildTruncOrBitCast, LLVMGetValueKind, LLVMInt1Type, LLVMIsThreadLocal},
    prelude::LLVMValueRef,
    LLVMValueKind,
};
use std::ffi::CStr;

fn is_thread_local(value: LLVMValueRef) -> bool {
    unsafe {
        LLVMGetValueKind(value) == LLVMValueKind::LLVMGlobalVariableValueKind
            && LLVMIsThreadLocal(value) != 0
    }
}

pub fn emit_load_of_scalar(
    builder: &Builder,
    address: &Address,
    volatility: Volatility,
    ir_type: &ir::Type,
) -> LLVMValueRef {
    let address = if is_thread_local(address.base_pointer()) {
        todo!("thread locals in emit_load_of_scalar not supported yet")
    } else {
        address
    };

    match ir_type {
        ir::Type::Vector(_) => todo!("vector types in emit_load_of_scalar not supported yet"),
        ir::Type::Atomic(_) => todo!("atomic types in emit_load_of_scalar not supported yet"),
        _ => (),
    }

    let load = builder.load(address, volatility);
    emit_from_mem(builder, load, ir_type)
}

pub fn emit_from_mem(builder: &Builder, value: LLVMValueRef, ir_type: &ir::Type) -> LLVMValueRef {
    match ir_type {
        ir::Type::Boolean => unsafe {
            LLVMBuildTruncOrBitCast(builder.get(), value, LLVMInt1Type(), cstr!("").as_ptr())
        },
        _ => value,
    }
}

pub fn build_mem_tmp(
    ctx: &BackendCtx,
    builder: &Builder,
    alloca_point: LLVMValueRef,
    ir_type: &ir::Type,
    name: &CStr,
) -> Result<RawAddress, BackendError> {
    let alignment = ctx.type_layout_cache.get(ir_type).alignment;
    build_mem_tmp_with_alignment(ctx, builder, alloca_point, ir_type, alignment, name)
}

pub fn build_mem_tmp_with_alignment(
    ctx: &BackendCtx,
    builder: &Builder,
    alloca_point: LLVMValueRef,
    ir_type: &ir::Type,
    alignment: ByteUnits,
    name: &CStr,
) -> Result<RawAddress, BackendError> {
    let backend_type = unsafe { to_backend_type(ctx.for_making_type(), ir_type)? };

    Ok(build_tmp_alloca_address(
        builder,
        alloca_point,
        backend_type,
        alignment,
        name,
        None,
    ))
}
