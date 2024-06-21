use super::{structure::to_backend_struct_type, ctx::BackendContext, BackendError};
use crate::{ir, resolved::StructureRef};
use llvm_sys::{
    core::{
        LLVMArrayType2, LLVMDoubleType, LLVMFloatType, LLVMFunctionType, LLVMInt16Type,
        LLVMInt1Type, LLVMInt32Type, LLVMInt64Type, LLVMInt8Type, LLVMPointerType, LLVMStructType,
        LLVMVoidType,
    },
    prelude::LLVMTypeRef,
};
use std::collections::HashSet;

pub unsafe fn to_backend_type(
    ctx: &BackendContext,
    ir_type: &ir::Type,
    visited: &mut HashSet<StructureRef>,
) -> Result<LLVMTypeRef, BackendError> {
    Ok(match ir_type {
        ir::Type::Void => LLVMVoidType(),
        ir::Type::Boolean => LLVMInt1Type(),
        ir::Type::S8 | ir::Type::U8 => LLVMInt8Type(),
        ir::Type::S16 | ir::Type::U16 => LLVMInt16Type(),
        ir::Type::S32 | ir::Type::U32 => LLVMInt32Type(),
        ir::Type::S64 | ir::Type::U64 => LLVMInt64Type(),
        ir::Type::F32 => LLVMFloatType(),
        ir::Type::F64 => LLVMDoubleType(),
        ir::Type::Pointer(to) => LLVMPointerType(to_backend_type(ctx, to, visited)?, 0),
        ir::Type::UntypedEnum(_) => panic!("Cannot convert untyped enum to backend type"),
        ir::Type::AnonymousComposite(composite) => {
            let mut subtypes = to_backend_types(ctx, &composite.subtypes, visited)?;

            LLVMStructType(
                subtypes.as_mut_ptr(),
                subtypes.len() as u32,
                composite.is_packed.into(),
            )
        }
        ir::Type::Structure(structure_ref) => to_backend_struct_type(ctx, structure_ref, visited)?,
        ir::Type::FunctionPointer => LLVMPointerType(LLVMInt8Type(), 0),
        ir::Type::FixedArray(fixed_array) => {
            let element_type = to_backend_type(ctx, &fixed_array.inner, visited)?;
            LLVMArrayType2(element_type, fixed_array.size)
        }
    })
}

pub unsafe fn to_backend_types(
    ctx: &BackendContext,
    ir_types: &[ir::Type],
    visited: &mut HashSet<StructureRef>,
) -> Result<Vec<LLVMTypeRef>, BackendError> {
    let mut results = Vec::with_capacity(ir_types.len());

    for ir_type in ir_types.iter() {
        results.push(to_backend_type(ctx, ir_type, visited)?);
    }

    Ok(results)
}

pub unsafe fn get_function_type(
    ctx: &BackendContext,
    function: &ir::Function,
) -> Result<LLVMTypeRef, BackendError> {
    get_function_pointer_type(
        ctx,
        &function.parameters[..],
        &function.return_type,
        function.is_cstyle_variadic,
    )
}

pub unsafe fn get_function_pointer_type(
    ctx: &BackendContext,
    parameters: &[ir::Type],
    return_type: &ir::Type,
    is_cstyle_variadic: bool,
) -> Result<LLVMTypeRef, BackendError> {
    let mut visited = HashSet::default();
    let return_type = to_backend_type(ctx, &return_type, &mut visited)?;
    let mut parameters = to_backend_types(ctx, &parameters, &mut visited)?;
    let is_vararg = if is_cstyle_variadic { 1 } else { 0 };

    Ok(LLVMFunctionType(
        return_type,
        parameters.as_mut_ptr(),
        parameters.len().try_into().unwrap(),
        is_vararg,
    ))
}
