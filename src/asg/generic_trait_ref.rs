use super::{TraitRef, Type};

#[derive(Clone, Debug)]
pub struct GenericTraitRef {
    pub trait_ref: TraitRef,
    pub args: Vec<Type>,
}
