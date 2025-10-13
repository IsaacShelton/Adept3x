use super::{FuncHead, TypeHead};
use crate::module_graph::ModuleRef;
use ast_workspace::TypeDeclRef;
use derive_more::{From, IsVariant};

/// A symbol declaration
#[derive(Copy, Clone, Debug, From, IsVariant)]
pub enum DeclHead<'env> {
    FuncLike(&'env FuncHead<'env>),
    TypeLike(DeclHeadTypeLike<'env>),
    ValueLike(ValueLikeRef<'env>),
}

#[derive(Copy, Clone, Debug)]
pub enum DeclHeadTypeLike<'env> {
    Type(&'env TypeHead<'env>),
}

impl<'env> DeclHeadTypeLike<'env> {
    pub fn arity(&self) -> usize {
        match self {
            DeclHeadTypeLike::Type(type_head) => type_head.arity,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, From)]
pub enum TypeLikeRef<'env> {
    Type(TypeDeclRef),
    Impl(ast_workspace::ImplRef),
    Namespace(&'env str, ModuleRef<'env>),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, From)]
pub enum ValueLikeRef<'env> {
    Dummy,
    Global(ast_workspace::GlobalRef),
    ExprAlias(ast_workspace::ExprAliasRef),
    Namespace(ModuleRef<'env>),
}
