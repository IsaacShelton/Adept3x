use super::{
    attribute::{add_func_attribute, create_enum_attribute},
    function_type::{to_backend_function_type, FunctionType},
};
use crate::llvm_backend::{
    abi::abi_function::ABIFunction,
    backend_type::{to_backend_type, to_backend_types},
    ctx::{BackendCtx, FunctionSkeleton},
    error::BackendError,
};
use cstr::cstr;
use llvm_sys::{
    core::{LLVMAddFunction, LLVMFunctionType, LLVMSetFunctionCallConv, LLVMSetLinkage},
    LLVMCallConv, LLVMLinkage,
};
use std::ffi::CString;

pub unsafe fn create_function_heads(ctx: &mut BackendCtx) -> Result<(), BackendError> {
    for (function_ref, function) in ctx.ir_module.functions.iter() {
        let mut abi_function = function
            .abide_abi
            .then(|| {
                let num_required = function.parameters.len();

                ABIFunction::new(
                    ctx,
                    function.parameters.iter(),
                    num_required,
                    &function.return_type,
                    function.is_cstyle_variadic,
                )
            })
            .transpose()?;

        let function_type = if let Some(abi_function) = &mut abi_function {
            to_backend_function_type(ctx, abi_function, function.is_cstyle_variadic)?
        } else {
            let mut parameters =
                to_backend_types(ctx.for_making_type(), function.parameters.iter())?;
            let return_type = to_backend_type(ctx.for_making_type(), &function.return_type)?;

            let pointer = LLVMFunctionType(
                return_type,
                parameters.as_mut_ptr(),
                parameters.len() as u32,
                function.is_cstyle_variadic as i32,
            );

            FunctionType {
                pointer,
                parameters,
                return_type,
                is_cstyle_variadic: function.is_cstyle_variadic,
            }
        };

        let name = CString::new(function.mangled_name.as_bytes()).unwrap();
        let skeleton = LLVMAddFunction(
            ctx.backend_module.get(),
            name.as_ptr(),
            function_type.pointer,
        );
        LLVMSetFunctionCallConv(skeleton, LLVMCallConv::LLVMCCallConv as u32);

        let nounwind = create_enum_attribute(cstr!("nounwind"), 0);
        add_func_attribute(skeleton, nounwind);

        if !function.is_foreign && !function.is_exposed {
            LLVMSetLinkage(skeleton, LLVMLinkage::LLVMPrivateLinkage);
        }

        ctx.func_skeletons.insert(
            *function_ref,
            FunctionSkeleton {
                function: skeleton,
                abi_function,
                function_type,
                ir_function_ref: *function_ref,
            },
        );
    }

    Ok(())
}
