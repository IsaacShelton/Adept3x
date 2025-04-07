use super::{HumanName, Params, Type, TypeParams};
use indexmap::IndexMap;
use source_files::Source;

#[derive(Clone, Debug)]
pub struct Trait {
    pub human_name: HumanName,
    pub source: Source,
    pub params: TypeParams,
    pub funcs: IndexMap<String, TraitFunc>,
}

#[derive(Clone, Debug)]
pub struct TraitFunc {
    pub params: Params,
    pub return_type: Type,
    pub source: Source,
}
