use crate::{
    data_units::ByteUnits,
    ir,
    llvm_backend::{
        address::Address,
        backend_type::{to_backend_mem_type, to_backend_type},
        builder::Builder,
        ctx::BackendCtx,
        error::BackendError,
        raw_address::RawAddress,
        target_data::TargetData,
    },
    target_info::type_layout::TypeLayoutCache,
};
use cstr::cstr;
use llvm_sys::{
    core::{
        LLVMBuildAlloca, LLVMBuildArrayAlloca, LLVMGetInsertBlock, LLVMPositionBuilderAtEnd,
        LLVMPositionBuilderBefore, LLVMSetAlignment,
    },
    prelude::{LLVMTypeRef, LLVMValueRef},
    target::LLVMPreferredAlignmentOfType,
};
use std::ffi::CStr;

pub fn build_default_align_tmp_alloca(
    target_data: &TargetData,
    builder: &Builder,
    alloca_insertion_point: LLVMValueRef,
    ty: LLVMTypeRef,
    name: &CStr,
) -> RawAddress {
    let alignment = ByteUnits::from(unsafe { LLVMPreferredAlignmentOfType(target_data.get(), ty) });
    build_tmp_alloca_address(builder, alloca_insertion_point, ty, alignment, name, None)
}

pub fn build_tmp(
    builder: &Builder,
    ctx: &BackendCtx,
    alloca_insertion_point: LLVMValueRef,
    ir_type: &ir::Type,
    name: Option<&CStr>,
) -> Result<RawAddress, BackendError> {
    let type_layout_cache = &ctx.type_layout_cache;
    let alignment = type_layout_cache.get(ir_type).alignment;

    Ok(build_tmp_alloca_address(
        builder,
        alloca_insertion_point,
        unsafe { to_backend_type(ctx.for_making_type(), ir_type)? },
        alignment,
        name.unwrap_or_else(|| cstr!("tmp")),
        None,
    ))
}

pub fn build_tmp_alloca_address(
    builder: &Builder,
    alloca_insertion_point: LLVMValueRef,
    ty: LLVMTypeRef,
    alignment: ByteUnits,
    name: &CStr,
    array_size: Option<LLVMValueRef>,
) -> RawAddress {
    let alloca = build_tmp_alloca_inst(builder, ty, name, array_size, alloca_insertion_point);
    unsafe { LLVMSetAlignment(alloca, alignment.bytes().try_into().unwrap()) };

    RawAddress {
        base: alloca,
        nullable: false,
        alignment,
        element_type: ty,
    }
}

pub fn build_tmp_alloca_inst(
    builder: &Builder,
    ty: LLVMTypeRef,
    name: &CStr,
    array_size: Option<LLVMValueRef>,
    alloca_insertion_point: LLVMValueRef,
) -> LLVMValueRef {
    let alloca = if let Some(array_size) = array_size {
        unsafe {
            let current_block = LLVMGetInsertBlock(builder.get());
            LLVMPositionBuilderBefore(builder.get(), alloca_insertion_point);

            let inserted = LLVMBuildArrayAlloca(builder.get(), ty, array_size, name.as_ptr());
            LLVMPositionBuilderAtEnd(builder.get(), current_block);
            inserted
        }
    } else {
        unsafe { LLVMBuildAlloca(builder.get(), ty, name.as_ptr()) }
    };

    alloca
}

pub fn make_natural_address_for_pointer(
    ctx: &BackendCtx,
    ptr: LLVMValueRef,
    ir_type: &ir::Type,
    alignment: Option<ByteUnits>,
    is_bitfield: Option<bool>,
) -> Result<Address, BackendError> {
    let alignment = match alignment {
        Some(ByteUnits::ZERO) | None => get_natural_type_alignment(&ctx.type_layout_cache, ir_type),
        Some(alignment) => alignment,
    };

    Ok(Address {
        base: RawAddress {
            base: ptr,
            nullable: false,
            alignment,
            element_type: unsafe {
                to_backend_mem_type(
                    ctx.for_making_type(),
                    &ctx.type_layout_cache,
                    ir_type,
                    is_bitfield.unwrap_or(false),
                )?
            },
        },
        offset: None,
    })
}

pub fn get_natural_type_alignment(
    type_layout_cache: &TypeLayoutCache,
    ir_type: &ir::Type,
) -> ByteUnits {
    type_layout_cache.get(ir_type).alignment
}
