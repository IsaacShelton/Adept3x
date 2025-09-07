use super::{
    attribute::{add_func_attribute, create_enum_attribute},
    function_type::{FunctionType, to_backend_func_type},
};
use crate::build_llvm_ir::{
    abi::abi_function::ABIFunction,
    backend_type::{to_backend_type, to_backend_types},
    ctx::{BackendCtx, FunctionSkeleton},
};
use data_units::ByteUnits;
use diagnostics::ErrorDiagnostic;
use llvm_sys::{
    LLVMCallConv, LLVMLinkage,
    core::{
        LLVMAddFunction, LLVMFunctionType, LLVMGetNamedFunction, LLVMSetFunctionCallConv,
        LLVMSetLinkage,
    },
};
use std::{ffi::CString, ptr::null_mut};

pub unsafe fn create_func_heads(ctx: &mut BackendCtx) -> Result<(), ErrorDiagnostic> {
    for (func_ref, func) in ctx.ir_module.funcs.iter() {
        let mut abi_func = func
            .abide_abi
            .then(|| {
                let num_required = func.params.len();

                ABIFunction::new(
                    ctx,
                    func.params.iter(),
                    num_required,
                    &func.return_type,
                    func.is_cstyle_variadic,
                )
            })
            .transpose()?;

        let max_vector_width = abi_func
            .as_ref()
            .map(|abi_function| abi_function.head_max_vector_width)
            .unwrap_or(ByteUnits::ZERO);

        let function_type = if let Some(abi_function) = &mut abi_func {
            to_backend_func_type(ctx, abi_function, func.is_cstyle_variadic)?
        } else {
            let mut parameters = to_backend_types(ctx.for_making_type(), func.params.iter())?;
            let return_type = to_backend_type(ctx.for_making_type(), &func.return_type)?;

            let pointer = LLVMFunctionType(
                return_type,
                parameters.as_mut_ptr(),
                parameters.len() as u32,
                func.is_cstyle_variadic as i32,
            );

            FunctionType {
                pointer,
                parameters,
                return_type,
                is_cstyle_variadic: func.is_cstyle_variadic,
            }
        };

        let name = CString::new(func.mangled_name.as_bytes()).unwrap();

        let existing = if !func.ownership.should_mangle() {
            LLVMGetNamedFunction(ctx.backend_module.get(), name.as_ptr())
        } else {
            null_mut()
        };

        let skeleton = if existing.is_null() {
            let skeleton = LLVMAddFunction(
                ctx.backend_module.get(),
                name.as_ptr(),
                function_type.pointer,
            );

            LLVMSetFunctionCallConv(skeleton, LLVMCallConv::LLVMCCallConv as u32);

            let nounwind = create_enum_attribute(c"nounwind", 0);
            add_func_attribute(skeleton, nounwind);

            if func.ownership.should_mangle() {
                LLVMSetLinkage(skeleton, LLVMLinkage::LLVMPrivateLinkage);
            }

            skeleton
        } else {
            if !func.ownership.should_mangle() {
                LLVMSetLinkage(existing, LLVMLinkage::LLVMExternalLinkage);
            }

            existing
        };

        ctx.func_skeletons.insert(
            func_ref,
            FunctionSkeleton {
                function: skeleton,
                abi_function: abi_func,
                function_type,
                ir_func_ref: func_ref,
                max_vector_width,
            },
        );
    }

    Ok(())
}
