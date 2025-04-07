use super::{Params, Type, TypeParams};
use attributes::Privacy;
use source_files::Source;

#[derive(Clone, Debug)]
pub struct Trait {
    pub name: String,
    pub params: TypeParams,
    pub source: Source,
    pub privacy: Privacy,
    pub funcs: Vec<TraitFunc>,
}

#[derive(Clone, Debug)]
pub struct TraitFunc {
    pub name: String,
    pub params: Params,
    pub return_type: Type,
    pub source: Source,
}
