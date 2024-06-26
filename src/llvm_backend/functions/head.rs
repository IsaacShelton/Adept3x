use crate::llvm_backend::{
    backend_type::{to_backend_type, to_backend_types},
    ctx::BackendCtx,
    error::BackendError,
};
use llvm_sys::{
    core::{LLVMAddFunction, LLVMFunctionType, LLVMSetFunctionCallConv, LLVMSetLinkage},
    LLVMCallConv, LLVMLinkage,
};
use std::ffi::CString;

pub unsafe fn create_function_heads(ctx: &mut BackendCtx) -> Result<(), BackendError> {
    for (function_ref, function) in ctx.ir_module.functions.iter() {
        let mut parameters = to_backend_types(ctx.for_making_type(), &function.parameters)?;
        let return_type = to_backend_type(ctx.for_making_type(), &function.return_type)?;

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
