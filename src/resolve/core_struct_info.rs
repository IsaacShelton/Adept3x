use super::error::{ResolveError, ResolveErrorKind};
use crate::{
    asg::{self, Asg, HumanName, StructRef},
    source_files::Source,
};

#[derive(Clone, Debug)]
pub struct CoreStructInfo<'a> {
    pub name: &'a HumanName,
    pub struct_ref: StructRef,
    pub arguments: &'a [asg::Type],
}

pub fn get_core_struct_info<'a, 'b>(
    asg: &'b Asg<'a>,
    ty: &'a asg::Type,
    source: Source,
) -> Result<CoreStructInfo<'b>, Option<ResolveError>> {
    match &asg
        .unalias(ty)
        .map_err(|e| ResolveErrorKind::from(e).at(source))
        .map_err(Some)?
        .kind
    {
        asg::TypeKind::Structure(name, struct_ref, arguments) => Ok(CoreStructInfo {
            name,
            struct_ref: *struct_ref,
            arguments: arguments.as_slice(),
        }),
        _ => Err(None),
    }
}
