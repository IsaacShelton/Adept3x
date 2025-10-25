use super::{EnumBody, StructBody, TypeAliasBody, trait_body::TraitBody};
use crate::module_graph::ModuleView;
use by_address::ByAddress;
use derivative::Derivative;
use derive_more::IsVariant;
use source_files::Source;

#[derive(Copy, Clone, Debug, Derivative)]
#[derivative(PartialEq, Eq, Hash)]
pub struct TypeHead<'env> {
    pub name: &'env str,
    pub arity: usize,
    pub rest: TypeHeadRest<'env>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TypeHeadRest<'env> {
    pub kind: TypeHeadRestKind<'env>,
    pub view: ByAddress<&'env ModuleView<'env>>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, IsVariant)]
pub enum TypeHeadRestKind<'env> {
    Struct(ByAddress<&'env ast::Struct>),
    Alias(ByAddress<&'env ast::TypeAlias>),
}

impl<'env> TypeHeadRestKind<'env> {
    pub fn source(&self) -> Source {
        match self {
            TypeHeadRestKind::Struct(item) => item.source,
            TypeHeadRestKind::Alias(item) => item.source,
        }
    }
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
