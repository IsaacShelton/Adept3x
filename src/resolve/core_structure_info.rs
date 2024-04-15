use super::error::{ResolveError, ResolveErrorKind};
use crate::{
    ast::Source,
    resolved::{self, MemoryManagement, StructureRef},
    source_file_cache::SourceFileCache,
};

pub fn get_core_structure_info<'a>(
    source_file_cache: &SourceFileCache,
    resolved_type: &'a resolved::Type,
    source: Source,
) -> Result<(&'a str, StructureRef, MemoryManagement), ResolveError> {
    match resolved_type {
        resolved::Type::PlainOldData(name, structure_ref) => {
            Ok((name, *structure_ref, resolved::MemoryManagement::None))
        }
        resolved::Type::ManagedStructure(name, structure_ref) => Ok((
            name,
            *structure_ref,
            resolved::MemoryManagement::ReferenceCounted,
        )),
        resolved::Type::Unsync(inner) => get_core_structure_info(source_file_cache, inner, source),
        _ => Err(ResolveError::new(
            source_file_cache,
            source,
            ResolveErrorKind::CannotCreateStructLiteralForNonPlainOldDataStructure {
                bad_type: resolved_type.to_string(),
            },
        )),
    }
}
