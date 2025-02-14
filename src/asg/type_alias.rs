use super::{HumanName, Type};
use crate::source_files::Source;
use indexmap::IndexSet;

#[derive(Clone, Debug)]
pub struct TypeAlias {
    pub human_name: HumanName,
    pub source: Source,
    pub params: IndexSet<String>,
    pub becomes: Type,
}
