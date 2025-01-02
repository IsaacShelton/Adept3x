use super::{HumanName, Params, Type};
use crate::source_files::Source;
use indexmap::{IndexMap, IndexSet};

#[derive(Clone, Debug)]
pub struct Trait {
    pub human_name: HumanName,
    pub source: Source,
    pub params: IndexSet<String>,
    pub funcs: IndexMap<String, TraitFunc>,
}

#[derive(Clone, Debug)]
pub struct TraitFunc {
    pub params: Params,
    pub return_type: Type,
    pub source: Source,
}
