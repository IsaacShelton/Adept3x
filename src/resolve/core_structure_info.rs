use super::error::{ResolveError, ResolveErrorKind};
use crate::{
    name::ResolvedName,
    resolved::{self, StructureRef},
    source_files::Source,
};

pub fn get_core_structure_info(
    resolved_type: &resolved::Type,
    source: Source,
) -> Result<(&ResolvedName, StructureRef), ResolveError> {
    match &resolved_type.kind {
        resolved::TypeKind::Structure(name, structure_ref) => Ok((name, *structure_ref)),
        _ => Err(ResolveErrorKind::CannotCreateStructLiteralForNonStructure {
            bad_type: resolved_type.to_string(),
        }
        .at(source)),
    }
}
