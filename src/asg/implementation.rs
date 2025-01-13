use super::{FuncRef, GenericTraitRef};
use crate::source_files::Source;
use indexmap::IndexMap;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Impl {
    pub name_params: IndexMap<String, ()>,
    pub target: GenericTraitRef,
    pub source: Source,
    pub body: HashMap<String, FuncRef>,
}
