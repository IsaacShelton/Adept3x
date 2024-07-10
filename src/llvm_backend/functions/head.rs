use crate::llvm_backend::{
    abi::{
        abi_function::ABIFunction,
        arch::{aarch64, Arch},
    },
    backend_type::{to_backend_type, to_backend_types},
    ctx::BackendCtx,
    error::BackendError,
    functions::params_mapping::ParamsMapping,
};
use llvm_sys::{
    core::{LLVMAddFunction, LLVMFunctionType, LLVMSetFunctionCallConv, LLVMSetLinkage},
    prelude::LLVMTypeRef,
    LLVMCallConv, LLVMLinkage,
};
use std::ffi::CString;

pub unsafe fn create_function_heads(ctx: &mut BackendCtx) -> Result<(), BackendError> {
    for (function_ref, function) in ctx.ir_module.functions.iter() {
        let (mut parameters, return_type) = if function.abide_abi {
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

            emit_prologue(ctx, &abi_function)
        } else {
            let parameters = to_backend_types(ctx.for_making_type(), function.parameters.iter())?;
            let return_type = to_backend_type(ctx.for_making_type(), &function.return_type)?;
            (parameters, return_type)
        };

        let name = CString::new(function.mangled_name.as_bytes()).unwrap();

        let function_type = LLVMFunctionType(
            return_type,
            parameters.as_mut_ptr(),
            parameters.len() as u32,
            function.is_cstyle_variadic as i32,
        );

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
    let _params_mapping = ParamsMapping::new(&ctx.type_layout_cache, abi_function, &ctx.ir_module);

    todo!("emit_prologue - {:#?}", abi_function);
}
