use super::{backend_type::to_backend_types, ctx::BackendContext, BackendError};
use crate::resolved::StructureRef;
use llvm_sys::{core::LLVMStructType, prelude::LLVMTypeRef};
use std::collections::HashSet;

pub unsafe fn to_backend_struct_type(
    ctx: &BackendContext,
    structure_key: &StructureRef,
    visited: &mut HashSet<StructureRef>,
) -> Result<LLVMTypeRef, BackendError> {
    // Ensure not infinite size
    if visited.contains(structure_key) {
        // TODO: Improve error message
        return Err(BackendError {
            message: "Recursive data structure".into(),
        });
    }

    // Get cached type or insert computed type
    ctx.structure_cache
        .cache
        .try_insert_cloned(*structure_key, |_| {
            let ir_structure = ctx
                .ir_module
                .structures
                .get(structure_key)
                .expect("referenced IR structure to exist");

            visited.insert(*structure_key);
            let mut subtypes = to_backend_types(ctx, &ir_structure.fields, visited)?;
            visited.remove(structure_key);

            Ok(LLVMStructType(
                subtypes.as_mut_ptr(),
                subtypes.len().try_into().unwrap(),
                ir_structure.is_packed.into(),
            ))
        })
}
