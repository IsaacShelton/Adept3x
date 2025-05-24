use super::{Params, Type, TypeParams};
use attributes::Privacy;
use indexmap::IndexMap;
use source_files::Source;

#[derive(Clone, Debug)]
pub struct TraitBody<'env> {
    pub params: TypeParams,
    pub funcs: IndexMap<&'env str, TraitFunc<'env>>,
    pub source: Source,
    pub privacy: Privacy,
}

#[derive(Clone, Debug)]
pub struct TraitFunc<'env> {
    pub params: Params<'env>,
    pub return_type: &'env Type<'env>,
    pub source: Source,
}
