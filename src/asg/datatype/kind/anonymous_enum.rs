use crate::{asg::Type, ast::EnumMember, source_files::Source};
use core::hash::Hash;
use indexmap::IndexMap;

#[derive(Clone, Debug)]
pub struct AnonymousEnum {
    pub backing_type: Type,
    pub members: IndexMap<String, EnumMember>,
    pub source: Source,
}

impl PartialEq for AnonymousEnum {
    fn eq(&self, other: &Self) -> bool {
        self.backing_type.eq(&other.backing_type) && self.members.eq(&other.members)
    }
}

impl Hash for AnonymousEnum {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.backing_type.hash(state);

        for (key, value) in self.members.iter() {
            key.hash(state);
            value.hash(state);
        }
    }
}
