use super::{backend_type::to_backend_types, ctx::ToBackendTypeCtx, BackendError};
use crate::ir::{self, StructRef};
use llvm_sys::{core::LLVMStructType, prelude::LLVMTypeRef};

pub unsafe fn to_backend_struct_type(
    ctx: &ToBackendTypeCtx<'_>,
    structure_key: StructRef,
) -> Result<LLVMTypeRef, BackendError> {
    // Ensure not infinite size
    if ctx.visited.borrow().contains(&structure_key) {
        // TODO: Improve error message
        return Err(BackendError {
            message: "Recursive data structure".into(),
        });
    }

    // Get cached type or insert computed type
    ctx.structure_cache
        .cache
        .try_insert_cloned(structure_key, |_| {
            let ir_structure = ctx.ir_module.structs.get(structure_key);

            ctx.visited.borrow_mut().insert(structure_key);
            let mut subtypes =
                to_backend_types(ctx, ir_structure.fields.iter().map(ir::Field::ir_type))?;

            ctx.visited.borrow_mut().remove(&structure_key);

            Ok(LLVMStructType(
                subtypes.as_mut_ptr(),
                subtypes.len().try_into().unwrap(),
                ir_structure.is_packed.into(),
            ))
        })
}
