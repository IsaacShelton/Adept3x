use super::{FuncRef, GenericTraitRef, ImplRef, TypeParams};
use attributes::Privacy;
use source_files::Source;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Impl {
    pub params: TypeParams,
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
