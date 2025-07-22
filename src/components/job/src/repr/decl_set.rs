use super::{Decl, TypeLikeRef, ValueLikeRef};
use ast_workspace::{FuncRef, TypeDeclRef};
use std_ext::SmallVec2;

/// A group of declarations under the same name
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct DeclSet(SmallVec2<Decl>);

impl<'env> DeclSet {
    pub fn push_unique(&mut self, decl: Decl) {
        self.0.push(decl);
    }

    // (old)
    pub fn type_decls(&self) -> impl Iterator<Item = TypeDeclRef> {
        self.0.iter().filter_map(|decl| match decl {
            Decl::TypeLike(TypeLikeRef::Type(type_decl_ref)) => Some(*type_decl_ref),
            _ => None,
        })
    }

    // (old)
    pub fn func_decls(&self) -> impl Iterator<Item = FuncRef> {
        self.0.iter().filter_map(|decl| match decl {
            Decl::FuncLike(func_ref) => Some(*func_ref),
            _ => None,
        })
    }

    pub fn type_likes(&self) -> impl Iterator<Item = TypeLikeRef> {
        self.0.iter().filter_map(|decl| match decl {
            Decl::TypeLike(type_like) => Some(*type_like),
            _ => None,
        })
    }

    pub fn func_likes(&self) -> impl Iterator<Item = FuncRef> {
        self.0.iter().filter_map(|decl| match decl {
            Decl::FuncLike(func_ref) => Some(*func_ref),
            _ => None,
        })
    }

    pub fn value_likes(&self) -> impl Iterator<Item = ValueLikeRef> {
        self.0.iter().filter_map(|decl| match decl {
            Decl::ValueLike(value_like) => Some(*value_like),
            _ => None,
        })
    }
}
