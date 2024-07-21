use super::{helpers::emit_load_of_scalar, ParamValues};
use crate::{
    data_units::ByteUnits,
    ir,
    llvm_backend::{
        abi::{
            abi_function::ABIParam,
            abi_type::{get_struct_field_types, get_struct_num_fields, is_struct_type},
            has_scalar_evaluation_kind,
        },
        address::Address,
        backend_type::to_backend_type,
        builder::{Builder, Volatility},
        ctx::{BackendCtx, FunctionSkeleton},
        error::BackendError,
        functions::{
            param_values::{helpers::build_mem_tmp_with_alignment, value::ParamValue},
            params_mapping::ParamRange,
            prologue::helpers::build_tmp_alloca_address,
        },
        raw_address::RawAddress,
        target_data::TargetData,
    },
};
use cstr::cstr;
use llvm_sys::{
    core::{
        LLVMBuildBitCast, LLVMConstInt, LLVMGetParam, LLVMGetPointerAddressSpace, LLVMGetTypeKind,
        LLVMInt64Type, LLVMInt8Type, LLVMTypeOf,
    },
    prelude::{LLVMTypeRef, LLVMValueRef},
    target::{LLVMByteOrder, LLVMByteOrdering, LLVMPreferredAlignmentOfType, LLVMStoreSizeOfType},
    LLVMTypeKind,
};

fn is_pointer_type(ty: LLVMTypeRef) -> bool {
    unsafe { LLVMGetTypeKind(ty) == LLVMTypeKind::LLVMPointerTypeKind }
}

fn is_integer_type(ty: LLVMTypeRef) -> bool {
    unsafe { LLVMGetTypeKind(ty) == LLVMTypeKind::LLVMIntegerTypeKind }
}

fn is_integer_or_pointer_type(ty: LLVMTypeRef) -> bool {
    is_integer_type(ty) || is_pointer_type(ty)
}

impl ParamValues {
    #[allow(clippy::too_many_arguments)]
    pub fn push_direct_or_extend(
        &mut self,
        builder: &Builder,
        ctx: &BackendCtx,
        skeleton: &FunctionSkeleton,
        param_range: ParamRange,
        ir_param_type: &ir::Type,
        alloca_point: LLVMValueRef,
        abi_param: &ABIParam,
    ) -> Result<(), BackendError> {
        assert!(abi_param.abi_type.kind.is_direct() || abi_param.abi_type.kind.is_extend());

        let argument =
            unsafe { LLVMGetParam(skeleton.function, param_range.start.try_into().unwrap()) };
        let desired_llvm_param_type =
            unsafe { to_backend_type(ctx.for_making_type(), ir_param_type)? };

        let coerce_to_type = abi_param.abi_type.coerce_to_type().flatten().unwrap();
        let offset_align = abi_param.abi_type.get_direct_offset_align().unwrap();

        apply_attributes(
            abi_param,
            desired_llvm_param_type,
            ir_param_type,
            param_range,
        );

        // Trivial argument value
        if !is_struct_type(coerce_to_type)
            && coerce_to_type == unsafe { to_backend_type(ctx.for_making_type(), ir_param_type)? }
            && offset_align.offset.is_zero()
        {
            return self.push_direct_trivial(
                builder,
                coerce_to_type,
                argument,
                param_range,
                desired_llvm_param_type,
            );
        }

        if ir_param_type.is_fixed_vector() {
            todo!("fixed vector types are not supported yet");
        }

        let is_struct = is_struct_type(coerce_to_type);
        let user_specified_alignment = ctx.type_layout_cache.get(ir_param_type).alignment;

        let alloca = build_mem_tmp_with_alignment(
            ctx,
            builder,
            alloca_point,
            ir_param_type,
            user_specified_alignment,
            cstr!(""),
        )?;

        let pointer =
            emit_address_at_offset(builder, ctx.target_data, abi_param, alloca.clone().into());

        // Flatten struct type if possible for better optimizations
        if abi_param.abi_type.can_be_flattened() == Some(true)
            && is_struct
            && get_struct_num_fields(coerce_to_type) > 1
        {
            let struct_size = ctx.target_data.abi_size_of_type(coerce_to_type);
            let pointer_element_size = ctx.target_data.abi_size_of_type(pointer.element_type());

            // NOTE: We don't support scalable SIMD vector types
            let source_size = struct_size;
            let destination_size = pointer_element_size;

            let address_to_store_into = if source_size < destination_size {
                pointer.with_element_type(coerce_to_type)
            } else {
                build_tmp_alloca_address(
                    builder,
                    alloca_point,
                    coerce_to_type,
                    alloca.alignment,
                    cstr!("coerce"),
                    None,
                )
                .into()
            };

            let elements = get_struct_field_types(coerce_to_type);
            assert_eq!(elements.len(), param_range.len());

            for (field_i, llvm_parameter_i) in param_range.iter().enumerate() {
                let argument = unsafe {
                    LLVMGetParam(skeleton.function, llvm_parameter_i.try_into().unwrap())
                };

                let element_pointer = builder.gep_struct(
                    ctx.target_data,
                    &address_to_store_into,
                    field_i,
                    Some(elements.as_slice()),
                );

                builder.store(argument, &element_pointer);
            }

            if source_size > destination_size {
                let destination_size = unsafe {
                    LLVMConstInt(
                        LLVMInt64Type(),
                        destination_size.try_into().unwrap(),
                        false as i32,
                    )
                };

                builder.memcpy(&pointer, &address_to_store_into, destination_size);
            }
        } else {
            assert_eq!(param_range.len(), 1);

            let argument =
                unsafe { LLVMGetParam(skeleton.function, param_range.start.try_into().unwrap()) };
            build_coerced_store(builder, ctx.target_data, argument, &pointer, alloca_point);
        }

        self.values
            .push(if has_scalar_evaluation_kind(ir_param_type) {
                ParamValue::Direct(emit_load_of_scalar(
                    builder,
                    &Address::from(alloca),
                    Volatility::Normal,
                    ir_param_type,
                ))
            } else {
                ParamValue::Indirect(alloca.into())
            });

        Ok(())
    }

    fn push_direct_trivial(
        &mut self,
        builder: &Builder,
        coerce_to_type: LLVMTypeRef,
        argument: LLVMValueRef,
        param_range: ParamRange,
        desired_llvm_param_type: LLVMTypeRef,
    ) -> Result<(), BackendError> {
        assert_eq!(param_range.len(), 1);
        let mut value = argument;

        // Ensure argument is correct type
        if unsafe { LLVMTypeOf(value) } != coerce_to_type {
            value = builder.bitcast(value, coerce_to_type);
        }

        if unsafe { LLVMTypeOf(value) } != desired_llvm_param_type {
            value = unsafe {
                LLVMBuildBitCast(
                    builder.get(),
                    value,
                    desired_llvm_param_type,
                    cstr!("").as_ptr(),
                )
            };
        }

        self.values.push(ParamValue::Direct(value));
        Ok(())
    }
}

fn apply_attributes(
    abi_param: &ABIParam,
    desired_llvm_param_type: LLVMTypeRef,
    _ir_param_type: &ir::Type,
    param_range: ParamRange,
) {
    let coerce_to_type = abi_param.abi_type.coerce_to_type().flatten().unwrap();
    let offset_align = abi_param.abi_type.get_direct_offset_align().unwrap();

    if offset_align.offset.is_zero()
        && is_pointer_type(desired_llvm_param_type)
        && is_pointer_type(coerce_to_type)
    {
        assert_eq!(param_range.len(), 1);
        eprintln!("warning: apply_attributes for function prologues does not apply attributes yet");

        // TODO: Apply attributes
        // TODO: Apply restrict?
    }
}

fn emit_address_at_offset(
    builder: &Builder,
    target_data: &TargetData,
    abi_param: &ABIParam,
    mut address: Address,
) -> Address {
    if let Some(offset) = abi_param.abi_type.get_direct_offset() {
        if !offset.is_zero() {
            address = address.with_element_type(unsafe { LLVMInt8Type() });
            address = builder.gep_in_bounds(target_data, &address, offset.bytes());
            address =
                address.with_element_type(abi_param.abi_type.coerce_to_type().flatten().unwrap());
        }
    }

    address
}

fn build_coerced_store(
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

    let destination = if is_struct_type(destination_type) {
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

    if is_pointer_type(source_type) && is_pointer_type(destination_type) {
        // NOTE: We don't support pointers with non-default address spaces yet
        assert_eq!(unsafe { LLVMGetPointerAddressSpace(source_type) }, unsafe {
            LLVMGetPointerAddressSpace(destination_type)
        });
    }

    if is_integer_or_pointer_type(source_type) && is_integer_or_pointer_type(destination_type) {
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
    let size = unsafe {
        LLVMConstInt(
            LLVMInt64Type(),
            destination_size.try_into().unwrap(),
            false as _,
        )
    };
    let tmp = Address::from(build_tmp_alloca_for_coerce(
        builder,
        target_data,
        source_type,
        destination.base.alignment,
        alloca_point,
    ));
    builder.store(source, &tmp);
    builder.memcpy(&destination, &tmp, size);
}

fn build_tmp_alloca_for_coerce(
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

fn coerce_integer_likes(
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

fn enter_struct_pointer_for_coerced_access(
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
