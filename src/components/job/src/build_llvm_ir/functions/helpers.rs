use crate::{
    build_llvm_ir::{
        abi::abi_type::ABIType,
        address::Address,
        backend_type::{to_backend_mem_type, to_backend_type},
        builder::{Builder, Volatility},
        ctx::BackendCtx,
        llvm_type_ref_ext::LLVMTypeRefExt,
        llvm_value_ref_ext::LLVMValueRefExt,
        raw_address::RawAddress,
        target_data::TargetData,
    },
    ir,
    target_layout::TypeLayoutCache,
};
use data_units::ByteUnits;
use diagnostics::ErrorDiagnostic;
use llvm_sys::{
    LLVMValueKind,
    core::{
        LLVMBuildAlloca, LLVMBuildArrayAlloca, LLVMBuildTruncOrBitCast, LLVMGetInsertBlock,
        LLVMGetPointerAddressSpace, LLVMGetValueKind, LLVMInt1Type, LLVMInt8Type,
        LLVMIsThreadLocal, LLVMPositionBuilderBefore, LLVMSetAlignment, LLVMTypeOf,
    },
    prelude::{LLVMTypeRef, LLVMValueRef},
    target::{LLVMByteOrder, LLVMByteOrdering, LLVMPreferredAlignmentOfType},
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

pub fn build_tmp<'env>(
    ctx: &BackendCtx<'_, 'env>,
    builder: &Builder<'env>,
    alloca_point: LLVMValueRef,
    ir_type: &'env ir::Type<'env>,
    name: Option<&CStr>,
) -> Result<RawAddress, ErrorDiagnostic> {
    let type_layout_cache = &ctx.type_layout_cache;
    let alignment = type_layout_cache.get(ir_type).alignment;

    Ok(build_tmp_alloca_address(
        builder,
        alloca_point,
        unsafe { to_backend_type(ctx.for_making_type(), ir_type)? },
        alignment,
        name.unwrap_or_else(|| c"tmp"),
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

pub fn build_mem_tmp<'env>(
    ctx: &BackendCtx<'_, 'env>,
    builder: &Builder<'env>,
    alloca_point: LLVMValueRef,
    ir_type: &'env ir::Type<'env>,
    name: &CStr,
) -> Result<RawAddress, ErrorDiagnostic> {
    let alignment = ctx.type_layout_cache.get(ir_type).alignment;
    build_mem_tmp_with_alignment(ctx, builder, alloca_point, ir_type, alignment, name)
}

pub fn build_mem_tmp_with_alignment<'env>(
    ctx: &BackendCtx<'_, 'env>,
    builder: &Builder<'env>,
    alloca_point: LLVMValueRef,
    ir_type: &'env ir::Type<'env>,
    alignment: ByteUnits,
    name: &CStr,
) -> Result<RawAddress, ErrorDiagnostic> {
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

pub fn build_mem_tmp_without_cast<'env>(
    ctx: &BackendCtx<'_, 'env>,
    builder: &Builder<'env>,
    alloca_point: LLVMValueRef,
    ir_type: &'env ir::Type<'env>,
    alignment: ByteUnits,
    name: &CStr,
) -> Result<RawAddress, ErrorDiagnostic> {
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

pub fn build_tmp_alloca_without_cast<'env>(
    builder: &Builder<'env>,
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

pub fn make_natural_address_for_pointer<'env>(
    ctx: &BackendCtx<'_, 'env>,
    ptr: LLVMValueRef,
    ir_type: &'env ir::Type<'env>,
    alignment: Option<ByteUnits>,
    is_bitfield: bool,
) -> Result<Address, ErrorDiagnostic> {
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

pub fn get_natural_type_alignment<'env>(
    type_layout_cache: &TypeLayoutCache<'env>,
    ir_type: &'env ir::Type<'env>,
) -> ByteUnits {
    type_layout_cache.get(ir_type).alignment
}

pub fn emit_address_at_offset<'a>(
    builder: &Builder,
    target_data: &TargetData,
    abi_type: &ABIType,
    address: &'a Address,
) -> Cow<'a, Address> {
    if let Some(offset) = abi_type.get_direct_offset() {
        if !offset.is_zero() {
            let mut address = address.with_element_type(unsafe { LLVMInt8Type() });
            address = builder.gep_in_bounds(target_data, &address, 0, offset.bytes());
            address = address.with_element_type(abi_type.coerce_to_type().flatten().unwrap());
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
    destination_size: ByteUnits,
) -> Address {
    // Try to reduce the range of the pointer access while still having
    // enough data to satisfy `destination_size`

    let field_types = source_struct_type.field_types();

    let Some(first_field_type) = field_types.first() else {
        return source_pointer.clone();
    };

    let first_field_size = target_data.store_size_of_type(*first_field_type);
    let source_size = target_data.store_size_of_type(source_struct_type);

    // Can't scale down anymore
    if first_field_size < destination_size && first_field_size < source_size {
        return source_pointer.clone();
    }

    // Otherwise, we only need data within the first element, so scale down
    let source_pointer =
        builder.gep_struct(target_data, source_pointer, 0, Some(field_types.as_slice()));

    // Recursively descend
    if source_pointer.element_type().is_struct() {
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

    if source_type.is_pointer() {
        if destination_type.is_pointer() {
            return builder.bitcast(source, destination_type);
        }

        let pointer_sized_int_type = target_data.pointer_sized_int_type();
        source = builder.ptr_to_int(source, pointer_sized_int_type);
    }

    let destination_int_type = if destination_type.is_pointer() {
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
    if destination_type.is_pointer() {
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
    build_tmp_alloca_address(builder, alloca_point, ty, alignment, c"", None)
}

pub fn build_coerced_load(
    ctx: &BackendCtx,
    builder: &Builder,
    source: &Address,
    desired_type: LLVMTypeRef,
    alloca_point: LLVMValueRef,
) -> LLVMValueRef {
    let mut source = Cow::Borrowed(source);
    let mut source_type = source.element_type();

    if source_type == desired_type {
        return builder.load(&source, Volatility::Normal);
    }

    let destination_size = ctx.target_data.abi_size_of_type(desired_type);

    if source_type.is_struct() {
        source = Cow::Owned(enter_struct_pointer_for_coerced_access(
            builder,
            ctx.target_data,
            &source,
            source_type,
            destination_size.try_into().unwrap(),
        ));

        source_type = source.element_type();
    }

    let source_size = ctx.target_data.abi_size_of_type(source_type);

    if desired_type.is_integer_or_pointer() && source_type.is_integer_or_pointer() {
        let value = builder.load(&source, Volatility::Normal);
        return coerce_integer_likes(builder, ctx.target_data, value, desired_type);
    }

    if source_size >= destination_size {
        return builder.load(&source.with_element_type(desired_type), Volatility::Normal);
    }

    let tmp = Address::from(build_tmp_alloca_for_coerce(
        builder,
        ctx.target_data,
        desired_type,
        source.base.alignment,
        alloca_point,
    ));

    builder.memcpy(&tmp, &source, LLVMValueRef::new_u64(source_size.bytes()));
    builder.load(&tmp, Volatility::Normal)
}

pub fn build_coerced_store(
    builder: &Builder,
    target_data: &TargetData,
    source: LLVMValueRef,
    destination: &Address,
    alloca_point: LLVMValueRef,
) {
    let source_type = unsafe { LLVMTypeOf(source) };
    let mut destination_type = destination.element_type();

    if source_type == destination_type {
        builder.store(source, destination);
        return;
    }

    let source_size = target_data.abi_size_of_type(source_type);

    let destination = if destination_type.is_struct() {
        let minimized_range = enter_struct_pointer_for_coerced_access(
            builder,
            target_data,
            destination,
            destination_type,
            source_size.try_into().unwrap(),
        );
        destination_type = destination.element_type();
        minimized_range
    } else {
        destination.clone()
    };

    if source_type.is_pointer() && destination_type.is_pointer() {
        // NOTE: We don't support pointers with non-default address spaces yet
        assert_eq!(unsafe { LLVMGetPointerAddressSpace(source_type) }, unsafe {
            LLVMGetPointerAddressSpace(destination_type)
        });
    }

    if source_type.is_integer_or_pointer() && destination_type.is_integer_or_pointer() {
        let source = coerce_integer_likes(builder, target_data, source, destination_type);
        builder.store(source, &destination);
        return;
    }

    let destination_size = target_data.abi_size_of_type(destination_type);

    if source_size <= destination_size {
        let destination = destination.with_element_type(source_type);
        builder.store(source, &destination);
        return;
    }

    // Coerce via memory
    let tmp = Address::from(build_tmp_alloca_for_coerce(
        builder,
        target_data,
        source_type,
        destination.base.alignment,
        alloca_point,
    ));

    builder.store(source, &tmp);

    builder.memcpy(
        &destination,
        &tmp,
        LLVMValueRef::new_u64(destination_size.bytes()),
    );
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

fn is_thread_local(value: LLVMValueRef) -> bool {
    unsafe {
        LLVMGetValueKind(value) == LLVMValueKind::LLVMGlobalVariableValueKind
            && LLVMIsThreadLocal(value) != 0
    }
}

pub fn emit_from_mem(builder: &Builder, value: LLVMValueRef, ir_type: &ir::Type) -> LLVMValueRef {
    match ir_type {
        ir::Type::Bool => unsafe {
            LLVMBuildTruncOrBitCast(builder.get(), value, LLVMInt1Type(), c"".as_ptr())
        },
        _ => value,
    }
}
