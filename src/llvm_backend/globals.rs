use super::{backend_type::to_backend_type, ctx::BackendCtx, BackendError};
use llvm_sys::{
    core::{
        LLVMAddGlobal, LLVMGetLinkage, LLVMGetNamedGlobal, LLVMGetThreadLocalMode, LLVMGetUndef,
        LLVMIsExternallyInitialized, LLVMSetExternallyInitialized, LLVMSetInitializer,
        LLVMSetLinkage, LLVMSetThreadLocalMode,
    },
    LLVMLinkage, LLVMThreadLocalMode,
};
use std::ffi::CString;

pub unsafe fn create_globals(ctx: &mut BackendCtx) -> Result<(), BackendError> {
    for (global_ref, global) in ctx.ir_module.globals.iter() {
        let backend_type = to_backend_type(ctx.for_making_type(), &global.ir_type)?;

        let name = CString::new(global.mangled_name.as_bytes()).unwrap();
        let existing = LLVMGetNamedGlobal(ctx.backend_module.get(), name.as_ptr());

        if !existing.is_null() {
            let existing_linkage = LLVMGetLinkage(existing);

            if (global.exposure.is_exposed() || global.is_foreign)
                && existing_linkage != LLVMLinkage::LLVMExternalLinkage
            {
                LLVMSetLinkage(existing, LLVMLinkage::LLVMExternalLinkage);
            }

            if global.is_thread_local
                && LLVMGetThreadLocalMode(existing) == LLVMThreadLocalMode::LLVMNotThreadLocal
            {
                LLVMSetThreadLocalMode(existing, LLVMThreadLocalMode::LLVMGeneralDynamicTLSModel);
            }

            if LLVMIsExternallyInitialized(existing) == 0 && !global.is_foreign {
                // In order to prevent aggressive optimizations from removing necessary internal global
                // variables, we'll mark them as externally-initialized
                LLVMSetExternallyInitialized(existing, true.into());
                LLVMSetInitializer(existing, LLVMGetUndef(backend_type));
            }

            // NOTE: We assume the two global variables have the same type here,
            // if they aren't, then we technically don't even to report it.
            ctx.globals.insert(*global_ref, existing);
            continue;
        }

        let backend_global = LLVMAddGlobal(ctx.backend_module.get(), backend_type, name.as_ptr());

        let linkage = if global.exposure.is_exposed() || global.is_foreign {
            LLVMLinkage::LLVMExternalLinkage
        } else {
            LLVMLinkage::LLVMInternalLinkage
        };

        LLVMSetLinkage(backend_global, linkage);

        if global.is_thread_local {
            LLVMSetThreadLocalMode(
                backend_global,
                LLVMThreadLocalMode::LLVMGeneralDynamicTLSModel,
            );
        }

        if !global.is_foreign {
            // In order to prevent aggressive optimizations from removing necessary internal global
            // variables, we'll mark them as externally-initialized
            LLVMSetExternallyInitialized(backend_global, true.into());
            LLVMSetInitializer(backend_global, LLVMGetUndef(backend_type));
        }

        ctx.globals.insert(*global_ref, backend_global);
    }

    Ok(())
}

pub unsafe fn create_static_variables() -> Result<(), BackendError> {
    Ok(())
}
