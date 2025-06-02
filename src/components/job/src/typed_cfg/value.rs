use super::Typed;
use crate::repr::Type;
use arena::Id;
use ast::NodeRef;

#[derive(Clone, Debug)]
pub struct Value<'env> {
    pub node_ref: NodeRef,
    pub cast_to: Cast<'env>,
}

impl<'env> Value<'env> {
    pub fn new(node_ref: NodeRef) -> Self {
        Self {
            node_ref,
            cast_to: Cast::Identity,
        }
    }
}

impl<'env> Value<'env> {
    pub fn ty<'a>(&'a self, types: &'a [Typed<'env>]) -> &'a Type<'env> {
        match &self.cast_to {
            Cast::Identity => &types[self.node_ref.into_raw().into_usize()].ty,
            Cast::Reinterpret(inner) => inner,
            Cast::Builtin(inner) => inner,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Cast<'env> {
    Identity,
    Reinterpret(Type<'env>),
    Builtin(Type<'env>),
}
