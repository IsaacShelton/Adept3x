use super::{backend_type::to_backend_types, ctx::ToBackendTypeCtx};
use crate::ir::StructRef;
use diagnostics::ErrorDiagnostic;
use llvm_sys::{core::LLVMStructType, prelude::LLVMTypeRef};

pub unsafe fn to_backend_struct_type<'env>(
    ctx: &ToBackendTypeCtx<'_, 'env>,
    struct_ref: StructRef<'env>,
) -> Result<LLVMTypeRef, ErrorDiagnostic> {
    // Ensure not infinite size
    if ctx.visited.borrow().contains(&struct_ref) {
        // TODO: Improve error message
        return Err(ErrorDiagnostic::plain("Recursive data structure"));
    }

    // Get cached type or insert computed type
    ctx.struct_cache.cache.try_insert_cloned(struct_ref, |_| {
        let ir_structure = &ctx.ir_module.structs[struct_ref];

        ctx.visited.borrow_mut().insert(struct_ref);
        let mut subtypes =
            to_backend_types(ctx, ir_structure.fields.iter().map(|field| &field.ir_type))?;

        ctx.visited.borrow_mut().remove(&struct_ref);

        Ok(LLVMStructType(
            subtypes.as_mut_ptr(),
            subtypes.len().try_into().unwrap(),
            ir_structure.is_packed.into(),
        ))
    })
}
