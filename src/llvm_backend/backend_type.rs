use super::{
    abi::{
        abi_function::ABIFunction,
        abi_type::{kinds::TypeExpansion, ABITypeKind},
    },
    ctx::{BackendCtx, ToBackendTypeCtx},
    functions::params_mapping::ParamsMapping,
    llvm_type_ref_ext::LLVMTypeRefExt,
    structure::to_backend_struct_type,
    BackendError,
};
use crate::{
    ir,
    llvm_backend::abi::abi_type::{kinds::get_type_expansion, Direct, Extend},
    target::type_layout::TypeLayoutCache,
};
use llvm_sys::{
    core::{
        LLVMArrayType2, LLVMDoubleType, LLVMFloatType, LLVMFunctionType, LLVMGetGlobalContext,
        LLVMInt16Type, LLVMInt1Type, LLVMInt32Type, LLVMInt64Type, LLVMInt8Type, LLVMPointerType,
        LLVMPointerTypeInContext, LLVMStructType, LLVMVoidType,
    },
    prelude::LLVMTypeRef,
};
use std::{borrow::Borrow, ptr::null_mut};

pub unsafe fn to_backend_type<'a>(
    ctx: impl Borrow<ToBackendTypeCtx<'a>>,
    ir_type: &ir::Type,
) -> Result<LLVMTypeRef, BackendError> {
    let ctx = ctx.borrow();

    Ok(match ir_type {
        ir::Type::Void => LLVMVoidType(),
        ir::Type::Boolean => LLVMInt1Type(),
        ir::Type::S8 | ir::Type::U8 => LLVMInt8Type(),
        ir::Type::S16 | ir::Type::U16 => LLVMInt16Type(),
        ir::Type::S32 | ir::Type::U32 => LLVMInt32Type(),
        ir::Type::S64 | ir::Type::U64 => LLVMInt64Type(),
        ir::Type::F32 => LLVMFloatType(),
        ir::Type::F64 => LLVMDoubleType(),
        ir::Type::Pointer(to) => LLVMPointerType(to_backend_type(ctx, to)?, 0),
        ir::Type::Union(_) => todo!("to_backend_type for ir::Type::Union"),
        ir::Type::AnonymousComposite(composite) => {
            let mut subtypes =
                to_backend_types(ctx, composite.fields.iter().map(ir::Field::ir_type))?;

            LLVMStructType(
                subtypes.as_mut_ptr(),
                subtypes.len() as u32,
                composite.is_packed.into(),
            )
        }
        ir::Type::Structure(structure_ref) => to_backend_struct_type(ctx, *structure_ref)?,
        ir::Type::FunctionPointer => LLVMPointerType(LLVMInt8Type(), 0),
        ir::Type::FixedArray(fixed_array) => {
            let element_type = to_backend_type(ctx, &fixed_array.inner)?;
            LLVMArrayType2(element_type, fixed_array.length)
        }
        ir::Type::Vector(_) => {
            todo!("to_backend_type for ir::Type::Vector")
        }
        ir::Type::Complex(_) => {
            todo!("to_backend_type for ir::Type::Complex")
        }
        ir::Type::Atomic(_) => {
            todo!("to_backend_type for ir::Type::Atomic")
        }
        ir::Type::IncompleteArray(_) => {
            todo!("to_backend_type for ir::Type::IncompleteArray")
        }
    })
}

pub unsafe fn to_backend_types<'a, 't>(
    ctx: impl Borrow<ToBackendTypeCtx<'a>>,
    ir_types: impl Iterator<Item = &'t ir::Type>,
) -> Result<Vec<LLVMTypeRef>, BackendError> {
    let estimated_count = ir_types.size_hint().0;
    let mut results = Vec::with_capacity(estimated_count);

    for ir_type in ir_types {
        results.push(to_backend_type(ctx.borrow(), ir_type)?);
    }

    Ok(results)
}

pub unsafe fn get_unabi_function_type<'a>(
    ctx: impl Borrow<ToBackendTypeCtx<'a>>,
    function: &ir::Function,
) -> Result<LLVMTypeRef, BackendError> {
    get_function_pointer_type(
        ctx.borrow(),
        &function.parameters[..],
        &function.return_type,
        function.is_cstyle_variadic,
    )
}

pub unsafe fn get_abi_function_type(
    ctx: &BackendCtx,
    function: &ir::Function,
    abi_function: &ABIFunction,
    params_mapping: &ParamsMapping,
) -> Result<LLVMTypeRef, BackendError> {
    let abi_return_info = &abi_function.return_type.abi_type;

    let return_type = match &abi_return_info.kind {
        ABITypeKind::Direct(direct) => direct.coerce_to_type.expect("filled direct return mode"),
        ABITypeKind::Extend(extend) => extend.coerce_to_type.expect("filled extend return mode"),
        ABITypeKind::Indirect(_) | ABITypeKind::Ignore => LLVMVoidType(),
        ABITypeKind::CoerceAndExpand(coerce_and_expand) => {
            coerce_and_expand.unpadded_coerce_and_expand_type
        }
        ABITypeKind::InAlloca(inalloca) => {
            if inalloca.sret {
                LLVMPointerType(
                    unsafe { to_backend_type(ctx.for_making_type(), &function.return_type)? },
                    0,
                )
            } else {
                LLVMVoidType()
            }
        }
        ABITypeKind::IndirectAliased(_) | ABITypeKind::Expand(_) => {
            panic!("invalid abi return mode")
        }
    };

    let mut arg_types = vec![null_mut(); params_mapping.llvm_arity()].into_boxed_slice();

    for (mapped_param, abi_param) in params_mapping
        .params()
        .iter()
        .zip(&abi_function.parameter_types)
    {
        let param_range = mapped_param.range();

        if let Some(padding_index) = mapped_param.padding_index() {
            arg_types[padding_index] = abi_param
                .abi_type
                .padding_type()
                .flatten()
                .expect("padding type");
        }

        match &abi_param.abi_type.kind {
            ABITypeKind::Direct(Direct {
                coerce_to_type,
                can_be_flattened,
                ..
            }) => {
                let arg_type = coerce_to_type.expect("filled in direct/extend for fty");

                if *can_be_flattened && arg_type.is_struct() {
                    let field_types = arg_type.field_types();

                    assert_eq!(field_types.len(), param_range.len());

                    for (llvm_arg_i, field_type) in
                        param_range.iter().zip(field_types.iter().copied())
                    {
                        arg_types[llvm_arg_i] = field_type;
                    }
                } else {
                    assert_eq!(param_range.len(), 1);
                    arg_types[param_range.start] = arg_type;
                }
            }
            ABITypeKind::Extend(Extend { coerce_to_type, .. }) => {
                assert_eq!(param_range.len(), 1);
                arg_types[param_range.start] = coerce_to_type.expect("filled in extend for fty");
            }
            ABITypeKind::Indirect(_) => {
                assert_eq!(param_range.len(), 1);
                arg_types[param_range.start] = LLVMPointerTypeInContext(LLVMGetGlobalContext(), 0);
            }
            ABITypeKind::IndirectAliased(indirect_aliased) => {
                assert_eq!(param_range.len(), 1);
                arg_types[param_range.start] = LLVMPointerTypeInContext(
                    LLVMGetGlobalContext(),
                    indirect_aliased.address_space,
                );
            }
            ABITypeKind::Expand(_) => {
                let mut llvm_arg_i_iterator = param_range.iter();
                expand_types(
                    ctx,
                    &abi_param.ir_type,
                    &mut llvm_arg_i_iterator,
                    &mut arg_types,
                )?
            }
            ABITypeKind::CoerceAndExpand(coerce_and_expand) => {
                let sequence = coerce_and_expand.expanded_type_sequence();
                assert_eq!(sequence.len(), param_range.len());

                for (llvm_arg_i, field_type) in param_range.iter().zip(sequence.iter().copied()) {
                    arg_types[llvm_arg_i] = field_type;
                }
            }
            ABITypeKind::InAlloca(_) | ABITypeKind::Ignore => assert_eq!(param_range.len(), 0),
        }
    }

    Ok(LLVMFunctionType(
        return_type,
        arg_types.as_mut_ptr(),
        arg_types.len().try_into().unwrap(),
        function.is_cstyle_variadic as _,
    ))
}

unsafe fn expand_types(
    ctx: &BackendCtx,
    ir_type: &ir::Type,
    llvm_arg_i_iterator: &mut impl Iterator<Item = usize>,
    arg_types: &mut [LLVMTypeRef],
) -> Result<(), BackendError> {
    let expansion = get_type_expansion(ir_type, &ctx.type_layout_cache, ctx.ir_module);

    match expansion {
        TypeExpansion::FixedArray(fixed_array) => {
            for _ in 0..fixed_array.length {
                expand_types(ctx, ir_type, llvm_arg_i_iterator, arg_types)?;
            }
        }
        TypeExpansion::Record(fields) => {
            for field in fields {
                expand_types(ctx, &field.ir_type, llvm_arg_i_iterator, arg_types)?;
            }
        }
        TypeExpansion::Complex(_) => todo!("expand_types for complex types not supported yet"),
        TypeExpansion::None => {
            arg_types[llvm_arg_i_iterator.next().expect("argument position")] =
                to_backend_type(ctx.for_making_type(), ir_type)?
        }
    }

    Ok(())
}

pub unsafe fn get_function_pointer_type<'a>(
    ctx: impl Borrow<ToBackendTypeCtx<'a>>,
    parameters: &[ir::Type],
    return_type: &ir::Type,
    is_cstyle_variadic: bool,
) -> Result<LLVMTypeRef, BackendError> {
    let ctx = ctx.borrow();
    let return_type = to_backend_type(ctx, return_type)?;
    let mut parameters = to_backend_types(ctx, parameters.iter())?;
    let is_vararg = if is_cstyle_variadic { 1 } else { 0 };

    Ok(LLVMFunctionType(
        return_type,
        parameters.as_mut_ptr(),
        parameters.len().try_into().unwrap(),
        is_vararg,
    ))
}

// `to_backend_mem_type` is similar to `to_backend_type`, except
// that it generates the type for the backing memory, instead of
// the passing representation.
// e.g., booleans are i1, but stored as i8 or i32
pub unsafe fn to_backend_mem_type<'a>(
    ctx: impl Borrow<ToBackendTypeCtx<'a>>,
    type_layout_cache: &TypeLayoutCache,
    ir_type: &ir::Type,
    is_bitfield: bool,
) -> Result<LLVMTypeRef, BackendError> {
    // NOTE: We don't support bitfields yet
    assert!(!is_bitfield);

    // NOTE: We don't support vector types yet
    assert!(ir_type.is_vector());

    if ir_type.is_boolean() {
        return Ok(LLVMTypeRef::new_int(type_layout_cache.get(ir_type).width));
    }

    to_backend_type(ctx, ir_type)
}
