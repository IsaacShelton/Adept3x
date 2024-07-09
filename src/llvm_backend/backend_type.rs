use super::{ctx::ToBackendTypeCtx, structure::to_backend_struct_type, BackendError};
use crate::ir;
use llvm_sys::{
    core::{
        LLVMArrayType2, LLVMDoubleType, LLVMFloatType, LLVMFunctionType, LLVMInt16Type,
        LLVMInt1Type, LLVMInt32Type, LLVMInt64Type, LLVMInt8Type, LLVMPointerType, LLVMStructType,
        LLVMVoidType,
    },
    prelude::LLVMTypeRef,
};
use std::borrow::Borrow;

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
        ir::Type::Structure(structure_ref) => to_backend_struct_type(ctx, structure_ref)?,
        ir::Type::FunctionPointer => LLVMPointerType(LLVMInt8Type(), 0),
        ir::Type::FixedArray(fixed_array) => {
            let element_type = to_backend_type(ctx, &fixed_array.inner)?;
            LLVMArrayType2(element_type, fixed_array.size)
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

pub unsafe fn get_function_type<'a>(
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

pub unsafe fn get_function_pointer_type<'a>(
    ctx: impl Borrow<ToBackendTypeCtx<'a>>,
    parameters: &[ir::Type],
    return_type: &ir::Type,
    is_cstyle_variadic: bool,
) -> Result<LLVMTypeRef, BackendError> {
    let ctx = ctx.borrow();
    let return_type = to_backend_type(ctx, &return_type)?;
    let mut parameters = to_backend_types(ctx, parameters.iter())?;
    let is_vararg = if is_cstyle_variadic { 1 } else { 0 };

    Ok(LLVMFunctionType(
        return_type,
        parameters.as_mut_ptr(),
        parameters.len().try_into().unwrap(),
        is_vararg,
    ))
}
