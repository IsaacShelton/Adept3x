use super::Resolved;
use crate::{
    cfg::{NodeId, NodeRef},
    repr::Type,
};
use arena::ArenaMap;

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

    pub fn new_with(node_ref: NodeRef, cast_to: Cast<'env>) -> Self {
        Self { node_ref, cast_to }
    }

    pub fn reinterpret(node_ref: NodeRef, to: Type<'env>) -> Self {
        Self {
            node_ref,
            cast_to: Cast::Reinterpret(to),
        }
    }

    pub fn builtin_cast(node_ref: NodeRef, to: Type<'env>) -> Self {
        Self {
            node_ref,
            cast_to: Cast::BuiltinCast(to),
        }
    }
}

#[derive(Clone, Debug)]
pub struct BuiltinTypes<'env> {
    pub bool: Type<'env>,
    pub i32: Type<'env>,
    pub u32: Type<'env>,
    pub i64: Type<'env>,
    pub u64: Type<'env>,
    pub f64: Type<'env>,
}

impl<'env> Value<'env> {
    pub fn ty<'a>(&'a self, types: &'a ArenaMap<NodeId, Resolved<'env>>) -> &'a Type<'env> {
        match &self.cast_to {
            Cast::Identity => &types.get(self.node_ref.into_raw()).unwrap().ty,
            Cast::Reinterpret(inner) => inner,
            Cast::BuiltinCast(inner) => inner,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Cast<'env> {
    Identity,
    Reinterpret(Type<'env>),
    BuiltinCast(Type<'env>),
}
