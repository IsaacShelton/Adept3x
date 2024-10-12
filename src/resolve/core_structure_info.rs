use super::error::ResolveError;
use crate::{
    name::ResolvedName,
    resolved::{self, StructureRef},
    source_files::Source,
};

pub fn get_core_structure_info(
    _resolved_type: &resolved::Type,
    _source: Source,
) -> Result<(&ResolvedName, StructureRef), ResolveError> {
    todo!("get_core_structure_info");

    /*
    match &resolved_type.kind {
        resolved::TypeKind::Structure(name, structure_ref) => Ok((name, *structure_ref)),
        _ => Err(ResolveErrorKind::CannotCreateStructLiteralForNonStructure {
            bad_type: resolved_type.to_string(),
        }
        .at(source)),
    }
    */
}
