use crate::{
    data_units::ByteUnits,
    ir,
    llvm_backend::{
        abi::{
            abi_function::ABIParam,
            abi_type::{get_struct_field_types, is_struct_type},
        },
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
        LLVMBuildAlloca, LLVMBuildArrayAlloca, LLVMGetInsertBlock, LLVMGetTypeKind, LLVMInt8Type,
        LLVMPositionBuilderBefore, LLVMSetAlignment, LLVMTypeOf,
    },
    prelude::{LLVMTypeRef, LLVMValueRef},
    target::{LLVMByteOrder, LLVMByteOrdering, LLVMPreferredAlignmentOfType, LLVMStoreSizeOfType},
    LLVMTypeKind,
};
use std::{borrow::Cow, ffi::CStr};

pub fn build_default_align_tmp_alloca(
    target_data: &TargetData,
    builder: &Builder,
    alloca_point: LLVMValueRef,
    ty: LLVMTypeRef,
    name: &CStr,
) -> RawAddress {
    let alignment = ByteUnits::from(unsafe { LLVMPreferredAlignmentOfType(target_data.get(), ty) });
    build_tmp_alloca_address(builder, alloca_point, ty, alignment, name, None)
}

pub fn build_tmp(
    builder: &Builder,
    ctx: &BackendCtx,
    alloca_point: LLVMValueRef,
    ir_type: &ir::Type,
    name: Option<&CStr>,
) -> Result<RawAddress, BackendError> {
    let type_layout_cache = &ctx.type_layout_cache;
    let alignment = type_layout_cache.get(ir_type).alignment;

    Ok(build_tmp_alloca_address(
        builder,
        alloca_point,
        unsafe { to_backend_type(ctx.for_making_type(), ir_type)? },
        alignment,
        name.unwrap_or_else(|| cstr!("tmp")),
        None,
    ))
}

pub fn build_tmp_alloca_address(
    builder: &Builder,
    alloca_point: LLVMValueRef,
    ty: LLVMTypeRef,
    alignment: ByteUnits,
    name: &CStr,
    array_size: Option<LLVMValueRef>,
) -> RawAddress {
    let alloca = build_tmp_alloca_inst(builder, ty, name, array_size, alloca_point);
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
    alloca_point: LLVMValueRef,
) -> LLVMValueRef {
    if let Some(array_size) = array_size {
        unsafe {
            let current_block = LLVMGetInsertBlock(builder.get());
            LLVMPositionBuilderBefore(builder.get(), alloca_point);

            let inserted = LLVMBuildArrayAlloca(builder.get(), ty, array_size, name.as_ptr());
            builder.position(current_block);
            inserted
        }
    } else {
        unsafe { LLVMBuildAlloca(builder.get(), ty, name.as_ptr()) }
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

pub fn build_mem_tmp_without_cast(
    builder: &Builder,
    ctx: &BackendCtx,
    alloca_point: LLVMValueRef,
    ir_type: &ir::Type,
    alignment: ByteUnits,
    name: &CStr,
) -> Result<RawAddress, BackendError> {
    Ok(build_tmp_alloca_without_cast(
        builder,
        alloca_point,
        unsafe {
            to_backend_mem_type(
                ctx.for_making_type(),
                &ctx.type_layout_cache,
                ir_type,
                false,
            )?
        },
        alignment,
        name,
        None,
    ))
}

pub fn build_tmp_alloca_without_cast(
    builder: &Builder,
    alloca_point: LLVMValueRef,
    ty: LLVMTypeRef,
    alignment: ByteUnits,
    name: &CStr,
    array_size: Option<LLVMValueRef>,
) -> RawAddress {
    let alloca = build_tmp_alloca_inst(builder, ty, name, array_size, alloca_point);
    unsafe { LLVMSetAlignment(alloca, alignment.bytes().try_into().unwrap()) };

    RawAddress {
        base: alloca,
        nullable: false,
        alignment,
        element_type: ty,
    }
}

pub fn make_natural_address_for_pointer(
    ctx: &BackendCtx,
    ptr: LLVMValueRef,
    ir_type: &ir::Type,
    alignment: Option<ByteUnits>,
    is_bitfield: bool,
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
                    is_bitfield,
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

pub fn emit_address_at_offset<'a>(
    builder: &Builder,
    target_data: &TargetData,
    abi_param: &ABIParam,
    address: &'a Address,
) -> Cow<'a, Address> {
    if let Some(offset) = abi_param.abi_type.get_direct_offset() {
        if !offset.is_zero() {
            let mut address = address.with_element_type(unsafe { LLVMInt8Type() });
            address = builder.gep_in_bounds(target_data, &address, offset.bytes());
            address =
                address.with_element_type(abi_param.abi_type.coerce_to_type().flatten().unwrap());
            return Cow::Owned(address);
        }
    }

    Cow::Borrowed(address)
}

pub fn enter_struct_pointer_for_coerced_access(
    builder: &Builder,
    target_data: &TargetData,
    source_pointer: &Address,
    source_struct_type: LLVMTypeRef,
    destination_size: u64,
) -> Address {
    // Try to reduce the range of the pointer access while still having
    // enough data to satisfy `destination_size`

    let field_types = get_struct_field_types(source_struct_type);

    let Some(first_field_type) = field_types.first() else {
        return source_pointer.clone();
    };

    let first_field_size = unsafe { LLVMStoreSizeOfType(target_data.get(), *first_field_type) };
    let source_size = unsafe { LLVMStoreSizeOfType(target_data.get(), source_struct_type) };

    // Can't scale down anymore
    if first_field_size < destination_size && first_field_size < source_size {
        return source_pointer.clone();
    }

    // Otherwise, we only need data within the first element, so scale down
    let source_pointer =
        builder.gep_struct(target_data, source_pointer, 0, Some(field_types.as_slice()));

    // Recursively descend
    if is_struct_type(source_pointer.element_type()) {
        enter_struct_pointer_for_coerced_access(
            builder,
            target_data,
            &source_pointer,
            source_struct_type,
            destination_size,
        )
    } else {
        source_pointer
    }
}

pub fn is_pointer_type(ty: LLVMTypeRef) -> bool {
    unsafe { LLVMGetTypeKind(ty) == LLVMTypeKind::LLVMPointerTypeKind }
}

pub fn is_integer_type(ty: LLVMTypeRef) -> bool {
    unsafe { LLVMGetTypeKind(ty) == LLVMTypeKind::LLVMIntegerTypeKind }
}

pub fn is_integer_or_pointer_type(ty: LLVMTypeRef) -> bool {
    is_integer_type(ty) || is_pointer_type(ty)
}

pub fn coerce_integer_likes(
    builder: &Builder,
    target_data: &TargetData,
    source: LLVMValueRef,
    destination_type: LLVMTypeRef,
) -> LLVMValueRef {
    let mut source = source;
    let source_type = unsafe { LLVMTypeOf(source) };

    if source_type == destination_type {
        return source;
    }

    if is_pointer_type(source_type) {
        if is_pointer_type(destination_type) {
            return builder.bitcast(source, destination_type);
        }

        let pointer_sized_int_type = target_data.pointer_sized_int_type();
        source = builder.ptr_to_int(source, pointer_sized_int_type);
    }

    let destination_int_type = if is_pointer_type(destination_type) {
        target_data.pointer_sized_int_type()
    } else {
        destination_type
    };

    if source_type != destination_int_type {
        // NOTE: We don't support big-endian targets (at least yet) ...
        // This is important, since we need to make sure treat the integer here
        // as if it was from a memory cast on the target machine.
        // This is trivial in the little-endian case, but less so for big-endian
        assert_eq!(
            unsafe { LLVMByteOrder(target_data.get()) },
            LLVMByteOrdering::LLVMLittleEndian
        );
        source = builder.int_cast(source, destination_int_type, false);
    }

    // Re-cast to pointer if desired destination type
    if is_pointer_type(destination_type) {
        builder.int_to_ptr(source, destination_type)
    } else {
        source
    }
}

pub fn build_tmp_alloca_for_coerce(
    builder: &Builder,
    target_data: &TargetData,
    ty: LLVMTypeRef,
    min_alignment: ByteUnits,
    alloca_point: LLVMValueRef,
) -> RawAddress {
    let preferred_alignment = unsafe { LLVMPreferredAlignmentOfType(target_data.get(), ty) };
    let alignment = min_alignment.max(ByteUnits::of(preferred_alignment.into()));
    build_tmp_alloca_address(builder, alloca_point, ty, alignment, cstr!(""), None)
}
