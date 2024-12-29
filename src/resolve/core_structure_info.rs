use super::error::{ResolveError, ResolveErrorKind};
use crate::{
    asg::{self, Asg, HumanName, StructureRef},
    source_files::Source,
};

#[derive(Clone, Debug)]
pub struct CoreStructInfo<'a> {
    pub name: &'a HumanName,
    pub structure_ref: StructureRef,
    pub arguments: &'a [asg::Type],
}

pub fn get_core_structure_info<'a, 'b>(
    asg: &'b Asg<'a>,
    resolved_type: &'a asg::Type,
    source: Source,
) -> Result<CoreStructInfo<'b>, Option<ResolveError>> {
    match &asg
        .unalias(resolved_type)
        .map_err(|e| ResolveErrorKind::from(e).at(source))
        .map_err(Some)?
        .kind
    {
        asg::TypeKind::Structure(name, structure_ref, arguments) => Ok(CoreStructInfo {
            name,
            structure_ref: *structure_ref,
            arguments: arguments.as_slice(),
        }),
        _ => Err(None),
    }
}
