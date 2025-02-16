use super::error::{ResolveError, ResolveErrorKind};
use crate::{
    asg::{self, Asg, HumanName, StructRef},
    source_files::Source,
};
use std::borrow::Cow;

#[derive(Clone, Debug)]
pub struct CoreStructInfo<'a> {
    pub name: Cow<'a, HumanName>,
    pub struct_ref: StructRef,
    pub arguments: Cow<'a, [asg::Type]>,
}

pub fn get_core_struct_info<'a, 'b>(
    asg: &'b Asg<'a>,
    ty: &'a asg::Type,
    source: Source,
) -> Result<CoreStructInfo<'b>, Option<ResolveError>> {
    let t = asg
        .unalias(ty)
        .map_err(|e| ResolveErrorKind::from(e).at(source))
        .map_err(Some)?;

    match t {
        Cow::Borrowed(t) => match &t.kind {
            asg::TypeKind::Structure(name, struct_ref, arguments) => Ok(CoreStructInfo {
                name: Cow::Borrowed(&name),
                struct_ref: *struct_ref,
                arguments: Cow::Borrowed(arguments.as_slice()),
            }),
            _ => Err(None),
        },
        Cow::Owned(t) => match &t.kind {
            asg::TypeKind::Structure(name, struct_ref, arguments) => Ok(CoreStructInfo {
                name: Cow::Owned(name.clone()),
                struct_ref: *struct_ref,
                arguments: Cow::Owned(arguments.as_slice().to_owned()),
            }),
            _ => Err(None),
        },
    }
}
