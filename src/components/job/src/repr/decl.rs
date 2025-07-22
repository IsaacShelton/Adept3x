use super::{FuncHead, TypeHead};
use ast_workspace::{FuncRef, TypeDeclRef};
use derive_more::From;

/// A symbol declaration
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, From)]
pub enum Decl {
    FuncLike(FuncRef),
    TypeLike(TypeLikeRef),
    ValueLike(ValueLikeRef),
}

/// A symbol declaration
#[derive(Clone, Debug, From)]
pub enum DeclHead<'env> {
    FuncLike(FuncRef, &'env FuncHead<'env>),
    TypeLike(TypeLikeRef, &'env TypeHead<'env>),
    ValueLike(ValueLikeRef),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, From)]
pub enum TypeLikeRef {
    Type(TypeDeclRef),
    Impl(ast_workspace::ImplRef),
    Namespace(ast_workspace::NamespaceRef),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, From)]
pub enum ValueLikeRef {
    Global(ast_workspace::GlobalRef),
    ExprAlias(ast_workspace::ExprAliasRef),
}
