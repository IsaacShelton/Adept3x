use super::{FuncRef, Type};
use crate::source_files::Source;
use indexmap::IndexMap;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Impl {
    pub name_params: IndexMap<String, ()>,
    pub ty: Type,
    pub source: Source,
    pub body: HashMap<String, Vec<FuncRef>>,
}
