use super::{Privacy, Type, TypeParams};
use crate::source_files::Source;

#[derive(Clone, Debug)]
pub struct TypeAlias {
    pub name: String,
    pub params: TypeParams,
    pub value: Type,
    pub source: Source,
    pub privacy: Privacy,
}
