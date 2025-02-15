use super::{Privacy, Type};
use crate::source_files::Source;
use indexmap::IndexSet;

#[derive(Clone, Debug)]
pub struct TypeAlias {
    pub name: String,
    pub params: IndexSet<String>,
    pub value: Type,
    pub source: Source,
    pub privacy: Privacy,
}
