use super::{super::abi::abi_type::Extend, params_mapping::ParamsMapping};
use crate::build_llvm_ir::{
    abi::{
        abi_function::ABIFunction,
        abi_type::{ABITypeKind, Expand},
    },
    ctx::BackendCtx,
    llvm_type_ref_ext::LLVMTypeRefExt,
};
use diagnostics::ErrorDiagnostic;
use llvm_sys::{
    LLVMTypeKind,
    core::{
        LLVMFunctionType, LLVMGetModuleContext, LLVMGetTypeKind, LLVMPointerTypeInContext,
        LLVMVoidType,
    },
    prelude::LLVMTypeRef,
};
use std::ptr::null_mut;

pub struct FunctionType {
    pub pointer: LLVMTypeRef,
    pub parameters: Vec<LLVMTypeRef>,
    pub return_type: LLVMTypeRef,
    pub is_cstyle_variadic: bool,
}

pub unsafe fn to_backend_func_type<'env>(
    ctx: &BackendCtx<'_, 'env>,
    abi_function: &mut ABIFunction<'env>,
    is_cstyle_variadic: bool,
) -> Result<FunctionType, ErrorDiagnostic> {
    // TODO: This should be memoized

    let default_pointer_type =
        unsafe { LLVMPointerTypeInContext(LLVMGetModuleContext(ctx.backend_module.get()), 0) };

    let return_type = match &abi_function.return_type.abi_type.kind {
        ABITypeKind::Direct(direct) => direct.coerce_to_type.unwrap(),
        ABITypeKind::Extend(extend) => extend.coerce_to_type.unwrap(),
        ABITypeKind::Ignore | ABITypeKind::Indirect(_) => unsafe { LLVMVoidType() },
        ABITypeKind::IndirectAliased(_) | ABITypeKind::Expand(_) => {
            panic!("invalid abi return type")
        }
        ABITypeKind::CoerceAndExpand(coerce_and_expand) => {
            coerce_and_expand.unpadded_coerce_and_expand_type
        }
        ABITypeKind::InAlloca(inalloca) => {
            if inalloca.sret {
                default_pointer_type
            } else {
                unsafe { LLVMVoidType() }
            }
        }
    };

    let mapping = ParamsMapping::new(ctx, &ctx.type_layout_cache, abi_function, ctx.ir_module);
    let mut parameters = vec![null_mut(); mapping.llvm_arity()];

    if let Some(sret_index) = mapping.sret_index() {
        parameters[sret_index] = default_pointer_type;
    }

    if let Some(inalloc_index) = mapping.inalloca_index() {
        parameters[inalloc_index] = default_pointer_type;
    }

    for (mapped_param, abi_param) in mapping
        .params()
        .iter()
        .zip(abi_function.parameter_types.iter())
    {
        if let Some(padding_index) = mapped_param.padding_index() {
            parameters[padding_index] = abi_param.abi_type.padding_type().flatten().unwrap();
        }

        let range = mapped_param.range();

        match &abi_param.abi_type.kind {
            ABITypeKind::Direct(direct) => {
                let coerced = direct.coerce_to_type.unwrap();

                // Flatten first-class aggregate types to scalars if possible
                // for better LLVM optimizations
                if direct.can_be_flattened
                    && unsafe { LLVMGetTypeKind(coerced) == LLVMTypeKind::LLVMStructTypeKind }
                {
                    let field_types = coerced.field_types();
                    assert_eq!(range.len(), field_types.len());

                    for (field_i, param_i) in range.iter().enumerate() {
                        parameters[param_i] = field_types[field_i];
                    }
                } else {
                    assert_eq!(range.len(), 1);
                    parameters[range.start] = coerced;
                }
            }
            ABITypeKind::Extend(Extend { coerce_to_type, .. }) => {
                let coerced = coerce_to_type.unwrap();
                assert_eq!(range.len(), 1);
                parameters[range.start] = coerced;
            }
            ABITypeKind::Indirect(_) | ABITypeKind::IndirectAliased(_) => {
                assert_eq!(range.len(), 1);
                parameters[range.start] = default_pointer_type;
            }
            ABITypeKind::Expand(_) => {
                let expanded = Expand::expand(ctx, &abi_param.ir_type)?;
                assert_eq!(expanded.len(), range.len());

                for (param_i, element) in range.iter().zip(expanded.iter().copied()) {
                    parameters[param_i] = element;
                }
            }
            ABITypeKind::CoerceAndExpand(coerce_and_expand) => {
                let expanded = coerce_and_expand.expanded_type_sequence();
                assert_eq!(expanded.len(), range.len());

                for (param_i, element) in range.iter().zip(expanded.iter().copied()) {
                    parameters[param_i] = element;
                }
            }
            ABITypeKind::Ignore | ABITypeKind::InAlloca(_) => assert_eq!(range.len(), 0),
        }
    }
    let pointer = LLVMFunctionType(
        return_type,
        parameters.as_mut_ptr(),
        parameters.len().try_into().unwrap(),
        is_cstyle_variadic as i32,
    );

    Ok(FunctionType {
        pointer,
        parameters,
        return_type,
        is_cstyle_variadic,
    })
}
