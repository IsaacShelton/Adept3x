use crate::repr::{Type, TypeKind, UnaliasedType};
use primitives::{FloatSize, IntegerBits, IntegerSign};
use source_files::Source;

#[derive(Clone, Debug)]
pub struct BuiltinTypes<'env> {
    pub void: Type<'env>,
    pub null: Type<'env>,
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
            null: TypeKind::NullLiteral.at(Source::internal()),
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

    pub fn null(&'env self) -> UnaliasedType<'env> {
        UnaliasedType(&self.null)
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
