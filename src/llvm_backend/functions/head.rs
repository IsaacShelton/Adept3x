use crate::llvm_backend::{
    abi::{
        abi_function::ABIFunction,
        abi_type::{get_struct_field_types, ABITypeKind, Expand, Extend},
        arch::{aarch64, Arch},
    },
    backend_type::{to_backend_type, to_backend_types},
    ctx::BackendCtx,
    error::BackendError,
    functions::params_mapping::ParamsMapping,
};
use llvm_sys::{
    core::{
        LLVMAddFunction, LLVMFunctionType, LLVMGetModuleContext, LLVMGetTypeKind,
        LLVMPointerTypeInContext, LLVMSetFunctionCallConv, LLVMSetLinkage, LLVMVoidType,
    },
    prelude::LLVMTypeRef,
    LLVMCallConv, LLVMLinkage, LLVMTypeKind,
};
use std::{ffi::CString, ptr::null_mut};

pub unsafe fn to_backend_function_type(
    ctx: &BackendCtx,
    abi_function: ABIFunction,
    is_cstyle_variadic: bool,
) -> Result<LLVMTypeRef, BackendError> {
    // TODO: This should be memoized

    let mut abi_function = abi_function;

    // Fill in default coerce type for return type
    abi_function
        .return_type
        .abi_type
        .coerce_to_type_if_missing(|| {
            to_backend_type(ctx.for_making_type(), &abi_function.return_type.ir_type)
        })?;

    // Fill in default coerce types for parameters
    for abi_param in abi_function.parameter_types.iter_mut() {
        abi_param.abi_type.coerce_to_type_if_missing(|| {
            to_backend_type(ctx.for_making_type(), &abi_param.ir_type)
        })?;
    }

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

    let mapping = ParamsMapping::new(&ctx.type_layout_cache, &abi_function, ctx.ir_module);
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
                    let field_types = get_struct_field_types(coerced);
                    assert_eq!(range.clone().count(), field_types.len());

                    for (field_i, param_i) in range.clone().enumerate() {
                        parameters[param_i] = field_types[field_i];
                    }
                } else {
                    assert_eq!(range.clone().count(), 1);
                    parameters[range.start] = coerced;
                }
            }
            ABITypeKind::Extend(Extend { coerce_to_type, .. }) => {
                let coerced = coerce_to_type.unwrap();
                assert_eq!(range.clone().count(), 1);
                parameters[range.start] = coerced;
            }
            ABITypeKind::Indirect(_) | ABITypeKind::IndirectAliased(_) => {
                assert_eq!(range.clone().count(), 1);
                parameters[range.clone().start] = default_pointer_type;
            }
            ABITypeKind::Expand(_) => {
                let expanded = Expand::expand(ctx, &abi_param.ir_type)?;
                assert_eq!(expanded.len(), range.clone().count());

                for (param_i, element) in range.zip(expanded.iter().copied()) {
                    parameters[param_i] = element;
                }
            }
            ABITypeKind::CoerceAndExpand(coerce_and_expand) => {
                let expanded = coerce_and_expand.expanded_type_sequence();
                assert_eq!(expanded.len(), range.clone().count());

                for (param_i, element) in range.zip(expanded.iter().copied()) {
                    parameters[param_i] = element;
                }
            }
            ABITypeKind::Ignore | ABITypeKind::InAlloca(_) => assert_eq!(range.clone().count(), 0),
        }
    }

    Ok(LLVMFunctionType(
        return_type,
        parameters.as_mut_ptr(),
        parameters.len().try_into().unwrap(),
        is_cstyle_variadic as i32,
    ))
}

pub unsafe fn create_function_heads(ctx: &mut BackendCtx) -> Result<(), BackendError> {
    for (function_ref, function) in ctx.ir_module.functions.iter() {
        let function_type = if function.abide_abi {
            let abi_function = ABIFunction::new(
                ctx.for_making_type(),
                Arch::AARCH64(aarch64::AARCH64 {
                    variant: aarch64::Variant::DarwinPCS,
                    target_info: &ctx.ir_module.target_info,
                    type_layout_cache: &ctx.type_layout_cache,
                    ir_module: &ctx.ir_module,
                    is_cxx_mode: false,
                }),
                &function.parameters[..],
                &function.return_type,
                function.is_cstyle_variadic,
            )?;

            to_backend_function_type(ctx, abi_function, function.is_cstyle_variadic)?
        } else {
            let mut parameters =
                to_backend_types(ctx.for_making_type(), function.parameters.iter())?;
            let return_type = to_backend_type(ctx.for_making_type(), &function.return_type)?;

            LLVMFunctionType(
                return_type,
                parameters.as_mut_ptr(),
                parameters.len() as u32,
                function.is_cstyle_variadic as i32,
            )
        };

        let name = CString::new(function.mangled_name.as_bytes()).unwrap();
        let skeleton = LLVMAddFunction(ctx.backend_module.get(), name.as_ptr(), function_type);
        LLVMSetFunctionCallConv(skeleton, LLVMCallConv::LLVMCCallConv as u32);

        if !function.is_foreign && !function.is_exposed {
            LLVMSetLinkage(skeleton, LLVMLinkage::LLVMPrivateLinkage);
        }

        ctx.func_skeletons.insert(function_ref.clone(), skeleton);
    }

    Ok(())
}

fn emit_prologue(ctx: &BackendCtx, abi_function: &ABIFunction) -> (Vec<LLVMTypeRef>, LLVMTypeRef) {
    let params_mapping = ParamsMapping::new(&ctx.type_layout_cache, abi_function, &ctx.ir_module);

    // let llvm_func = todo!();
    // assert_eq!(llvm_func.arg_size(), params_mapping.arg_size());

    todo!("params_mapping - {:#?}", params_mapping);
    todo!("emit_prologue - {:#?}", abi_function);
}
