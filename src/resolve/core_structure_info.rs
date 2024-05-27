use super::error::{ResolveError, ResolveErrorKind};
use crate::{
    ast::Source,
    resolved::{self, MemoryManagement, StructureRef},
};

pub fn get_core_structure_info<'a>(
    resolved_type: &'a resolved::Type,
    source: Source,
) -> Result<(&'a str, StructureRef, MemoryManagement), ResolveError> {
    match &resolved_type.kind {
        resolved::TypeKind::PlainOldData(name, structure_ref) => {
            Ok((name, *structure_ref, resolved::MemoryManagement::None))
        }
        resolved::TypeKind::ManagedStructure(name, structure_ref) => Ok((
            name,
            *structure_ref,
            resolved::MemoryManagement::ReferenceCounted,
        )),
        _ => Err(
            ResolveErrorKind::CannotCreateStructLiteralForNonPlainOldDataStructure {
                bad_type: resolved_type.to_string(),
            }
            .at(source),
        ),
    }
}
