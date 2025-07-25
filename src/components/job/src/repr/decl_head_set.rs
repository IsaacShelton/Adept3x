use super::{DeclHead, FuncHead, TypeHead, TypeLikeRef, ValueLikeRef};
use ast_workspace::FuncRef;
use std_ext::SmallVec2;

/// A group of declarations under the same name
#[derive(Clone, Debug, Default)]
pub struct DeclHeadSet<'env>(SmallVec2<DeclHead<'env>>);

impl<'env> DeclHeadSet<'env> {
    pub fn push(&mut self, decl_head: DeclHead<'env>) {
        self.0.push(decl_head);
    }

    pub fn type_likes(&self) -> impl Iterator<Item = (TypeLikeRef, &'env TypeHead<'env>)> {
        self.0.iter().filter_map(|decl_head| match decl_head {
            DeclHead::TypeLike(type_like, type_head) => Some((*type_like, *type_head)),
            _ => None,
        })
    }

    pub fn func_likes(&self) -> impl Iterator<Item = (FuncRef, &'env FuncHead<'env>)> {
        self.0.iter().filter_map(|decl_head| match decl_head {
            DeclHead::FuncLike(func_ref, func_head) => Some((*func_ref, *func_head)),
            _ => None,
        })
    }

    pub fn value_likes(&self) -> impl Iterator<Item = ValueLikeRef> {
        self.0.iter().filter_map(|decl_head| match decl_head {
            DeclHead::ValueLike(value_like) => Some(*value_like),
            _ => None,
        })
    }
}
