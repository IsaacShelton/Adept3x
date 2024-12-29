use crate::{ast::EnumMember, asg::Type, source_files::Source};
use indexmap::IndexMap;

#[derive(Clone, Debug)]
pub struct AnonymousEnum {
    pub ty: Box<Type>,
    pub source: Source,
    pub members: IndexMap<String, EnumMember>,
}

impl PartialEq for AnonymousEnum {
    fn eq(&self, other: &Self) -> bool {
        self.ty.eq(&other.ty) && self.members.eq(&other.members)
    }
}
