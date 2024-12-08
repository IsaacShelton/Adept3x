use super::error::{ResolveError, ResolveErrorKind};
use crate::{
    resolved::{self, HumanName, StructureRef},
    source_files::Source,
};

#[derive(Clone, Debug)]
pub struct CoreStructInfo<'a> {
    pub name: &'a HumanName,
    pub structure_ref: StructureRef,
    pub arguments: &'a [resolved::Type],
}

pub fn get_core_structure_info<'a, 'b>(
    resolved_ast: &'b resolved::Ast<'a>,
    resolved_type: &'a resolved::Type,
    source: Source,
) -> Result<CoreStructInfo<'b>, Option<ResolveError>> {
    match &resolved_ast
        .unalias(resolved_type)
        .map_err(|e| ResolveErrorKind::from(e).at(source))
        .map_err(Some)?
        .kind
    {
        resolved::TypeKind::Structure(name, structure_ref, arguments) => Ok(CoreStructInfo {
            name,
            structure_ref: *structure_ref,
            arguments: arguments.as_slice(),
        }),
        _ => Err(None),
    }
}
