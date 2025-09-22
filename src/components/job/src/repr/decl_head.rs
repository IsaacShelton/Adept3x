use super::{FuncHead, TypeHead};
use crate::module_graph::ModuleRef;
use ast_workspace::TypeDeclRef;
use derive_more::From;

/// A symbol declaration
#[derive(Copy, Clone, Debug, From)]
pub enum DeclHead<'env> {
    FuncLike(&'env FuncHead<'env>),
    TypeLike(DeclHeadTypeLike<'env>),
    ValueLike(ValueLikeRef),
}

#[derive(Copy, Clone, Debug)]
pub enum DeclHeadTypeLike<'env> {
    Type(&'env TypeHead<'env>),
    Namespace(&'env str, ModuleRef<'env>),
}

impl<'env> DeclHeadTypeLike<'env> {
    pub fn arity(&self) -> usize {
        match self {
            DeclHeadTypeLike::Type(type_head) => type_head.arity,
            DeclHeadTypeLike::Namespace(_, _) => 0,
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
pub enum ValueLikeRef {
    Dummy,
    Global(ast_workspace::GlobalRef),
    ExprAlias(ast_workspace::ExprAliasRef),
}
