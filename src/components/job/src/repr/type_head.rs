use super::{EnumBody, StructBody, TypeAliasBody, trait_body::TraitBody};
use crate::module_graph::ModuleView;
use by_address::ByAddress;
use derivative::Derivative;

#[derive(Clone, Debug, Derivative)]
#[derivative(PartialEq, Eq, Hash)]
pub struct TypeHead<'env> {
    pub name: &'env str,
    pub arity: usize,

    #[derivative(Hash = "ignore")]
    #[derivative(PartialEq = "ignore")]
    pub rest: TypeHeadRest<'env>,
}

#[derive(Clone, Debug)]
pub struct TypeHeadRest<'env> {
    pub kind: TypeHeadRestKind<'env>,
    pub view: &'env ModuleView<'env>,
}

#[derive(Clone, Debug)]
pub enum TypeHeadRestKind<'env> {
    Struct(ByAddress<&'env ast::Struct>),
}

impl<'env> TypeHead<'env> {
    pub fn new(name: &'env str, arity: usize, rest: TypeHeadRest<'env>) -> Self {
        Self { name, arity, rest }
    }
}

#[derive(Clone, Debug)]
pub enum TypeBody<'env> {
    Struct(StructBody<'env>),
    Enum(EnumBody<'env>),
    TypeAlias(TypeAliasBody<'env>),
    Trait(TraitBody<'env>),
}
