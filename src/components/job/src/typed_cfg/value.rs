use super::Resolved;
use crate::{
    cfg::{NodeId, NodeRef},
    repr::{Type, TypeKind, UnaliasedType},
};
use arena::ArenaMap;
use primitives::{FloatSize, IntegerBits, IntegerSign};
use source_files::Source;

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

    pub fn reinterpret(node_ref: NodeRef, to: UnaliasedType<'env>) -> Self {
        Self {
            node_ref,
            cast_to: Cast::Reinterpret(to),
        }
    }

    pub fn builtin_cast(node_ref: NodeRef, to: UnaliasedType<'env>) -> Self {
        Self {
            node_ref,
            cast_to: Cast::BuiltinCast(to),
        }
    }
}

#[derive(Clone, Debug)]
pub struct BuiltinTypes<'env> {
    pub void: Type<'env>,
    pub bool: Type<'env>,
    pub i32: Type<'env>,
    pub u32: Type<'env>,
    pub i64: Type<'env>,
    pub u64: Type<'env>,
    pub f32: Type<'env>,
    pub f64: Type<'env>,
    pub never: Type<'env>,
}

impl<'env> Default for BuiltinTypes<'env> {
    fn default() -> Self {
        Self {
            void: TypeKind::Void.at(Source::internal()),
            bool: TypeKind::Boolean.at(Source::internal()),
            i32: TypeKind::BitInteger(IntegerBits::Bits32, IntegerSign::Signed)
                .at(Source::internal()),
            u32: TypeKind::BitInteger(IntegerBits::Bits32, IntegerSign::Unsigned)
                .at(Source::internal()),
            i64: TypeKind::BitInteger(IntegerBits::Bits64, IntegerSign::Signed)
                .at(Source::internal()),
            u64: TypeKind::BitInteger(IntegerBits::Bits64, IntegerSign::Unsigned)
                .at(Source::internal()),
            f32: TypeKind::Floating(FloatSize::Bits32).at(Source::internal()),
            f64: TypeKind::Floating(FloatSize::Bits64).at(Source::internal()),
            never: TypeKind::Never.at(Source::internal()),
        }
    }
}

impl<'env> BuiltinTypes<'env> {
    pub fn void(&'env self) -> UnaliasedType<'env> {
        UnaliasedType(&self.void)
    }

    pub fn bool(&'env self) -> UnaliasedType<'env> {
        UnaliasedType(&self.bool)
    }

    pub fn i32(&'env self) -> UnaliasedType<'env> {
        UnaliasedType(&self.i32)
    }

    pub fn u32(&'env self) -> UnaliasedType<'env> {
        UnaliasedType(&self.u32)
    }

    pub fn i64(&'env self) -> UnaliasedType<'env> {
        UnaliasedType(&self.i64)
    }

    pub fn u64(&'env self) -> UnaliasedType<'env> {
        UnaliasedType(&self.u64)
    }

    pub fn floating(&'env self, size: FloatSize) -> UnaliasedType<'env> {
        match size {
            FloatSize::Bits32 => self.f32(),
            FloatSize::Bits64 => self.f64(),
        }
    }

    pub fn f32(&'env self) -> UnaliasedType<'env> {
        UnaliasedType(&self.f32)
    }

    pub fn f64(&'env self) -> UnaliasedType<'env> {
        UnaliasedType(&self.f64)
    }

    pub fn never(&'env self) -> UnaliasedType<'env> {
        UnaliasedType(&self.never)
    }
}

impl<'env> Value<'env> {
    pub fn ty(
        &self,
        types: &ArenaMap<NodeId, Resolved<'env>>,
        builtin_types: &'env BuiltinTypes<'env>,
    ) -> UnaliasedType<'env> {
        match &self.cast_to {
            Cast::Identity => types
                .get(self.node_ref.into_raw())
                .map(|x| x.ty)
                .unwrap_or(builtin_types.never()),
            Cast::Reinterpret(inner) => *inner,
            Cast::BuiltinCast(inner) => *inner,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Cast<'env> {
    Identity,
    Reinterpret(UnaliasedType<'env>),
    BuiltinCast(UnaliasedType<'env>),
}
