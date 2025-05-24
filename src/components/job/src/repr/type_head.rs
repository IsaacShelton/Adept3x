use super::{EnumBody, StructBody, TypeAliasBody, trait_body::TraitBody};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TypeHead<'env> {
    pub name: &'env str,
    pub arity: usize,
}

impl<'env> TypeHead<'env> {
    pub fn new(name: &'env str, arity: usize) -> Self {
        Self { name, arity }
    }
}

#[derive(Clone, Debug)]
pub enum TypeBody<'env> {
    Struct(StructBody<'env>),
    Enum(EnumBody<'env>),
    TypeAlias(TypeAliasBody<'env>),
    Trait(TraitBody<'env>),
}
