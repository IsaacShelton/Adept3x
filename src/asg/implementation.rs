use super::{FuncRef, GenericTraitRef, ImplRef};
use crate::{ast::Privacy, source_files::Source};
use indexmap::IndexMap;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Impl {
    pub name_params: IndexMap<String, ()>,
    pub target: GenericTraitRef,
    pub source: Source,
    pub body: HashMap<String, FuncRef>,
}

#[derive(Clone, Debug)]
pub struct ImplDecl {
    pub impl_ref: ImplRef,
    pub source: Source,
    pub privacy: Privacy,
}
