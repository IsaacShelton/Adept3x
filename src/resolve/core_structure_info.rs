use super::error::{ResolveError, ResolveErrorKind};
use crate::{
    resolved::{self, HumanName, StructureRef},
    source_files::Source,
};

pub fn get_core_structure_info<'a, 'b>(
    resolved_ast: &'b resolved::Ast<'a>,
    resolved_type: &'a resolved::Type,
    source: Source,
) -> Result<(&'b HumanName, StructureRef, &'b [resolved::Type]), ResolveError> {
    match &resolved_ast
        .unalias(resolved_type)
        .map_err(|e| ResolveErrorKind::from(e).at(source))?
        .kind
    {
        resolved::TypeKind::Structure(name, structure_ref, parameters) => {
            Ok((name, *structure_ref, parameters.as_slice()))
        }
        _ => Err(ResolveErrorKind::CannotCreateStructLiteralForNonStructure {
            bad_type: resolved_type.to_string(),
        }
        .at(source)),
    }
}
