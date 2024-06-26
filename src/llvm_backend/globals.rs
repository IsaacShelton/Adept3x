use super::{backend_type::to_backend_type, ctx::BackendCtx, BackendError};
use llvm_sys::{
    core::{
        LLVMAddGlobal, LLVMGetUndef, LLVMSetExternallyInitialized, LLVMSetInitializer,
        LLVMSetLinkage, LLVMSetThreadLocal,
    },
    LLVMLinkage,
};
use std::{collections::HashSet, ffi::CString};

pub unsafe fn create_globals(ctx: &mut BackendCtx) -> Result<(), BackendError> {
    for (global_ref, global) in ctx.ir_module.globals.iter() {
        let backend_type = to_backend_type(ctx, &global.ir_type, &mut HashSet::default())?;

        let name = CString::new(global.mangled_name.as_bytes()).unwrap();
        let backend_global = LLVMAddGlobal(ctx.backend_module.get(), backend_type, name.as_ptr());

        let linkage = if global.is_foreign {
            LLVMLinkage::LLVMExternalLinkage
        } else {
            LLVMLinkage::LLVMInternalLinkage
        };

        LLVMSetLinkage(backend_global, linkage);

        if global.is_thread_local {
            LLVMSetThreadLocal(backend_global, true.into());
        }

        if !global.is_foreign {
            // In order to prevent aggressive optimizations from removing necessary internal global
            // variables, we'll mark them as externally-initialized
            LLVMSetExternallyInitialized(backend_global, true.into());
            LLVMSetInitializer(backend_global, LLVMGetUndef(backend_type));
        }

        ctx.globals.insert(global_ref.clone(), backend_global);
    }

    Ok(())
}

pub unsafe fn create_static_variables() -> Result<(), BackendError> {
    Ok(())
}
