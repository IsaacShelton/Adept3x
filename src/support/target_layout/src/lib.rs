mod alignment_requirement;
mod record_layout;
mod type_layout;
mod type_layout_cache;

use data_units::ByteUnits;
use primitives::CInteger;
pub use record_layout::*;
use target::{Target, TargetOsExt};
pub use type_layout::TypeLayout;
pub use type_layout_cache::TypeLayoutCache;

pub trait TargetLayout {
    fn pointer_layout(&self) -> TypeLayout;
    fn bool_layout(&self) -> TypeLayout;
    fn char_layout(&self) -> TypeLayout;
    fn short_layout(&self) -> TypeLayout;
    fn int_layout(&self) -> TypeLayout;
    fn long_layout(&self) -> TypeLayout;
    fn longlong_layout(&self) -> TypeLayout;
    fn float_layout(&self) -> TypeLayout;
    fn double_layout(&self) -> TypeLayout;
    fn size_layout(&self) -> TypeLayout;
    fn c_integer_bytes(&self, c_integer: CInteger) -> ByteUnits;
}

impl TargetLayout for Target {
    fn pointer_layout(&self) -> TypeLayout {
        TypeLayout::basic(ByteUnits::of(8))
    }

    fn bool_layout(&self) -> TypeLayout {
        TypeLayout::basic(ByteUnits::of(1))
    }

    fn char_layout(&self) -> TypeLayout {
        TypeLayout::basic(ByteUnits::of(1))
    }

    fn short_layout(&self) -> TypeLayout {
        TypeLayout::basic(ByteUnits::of(2))
    }

    fn int_layout(&self) -> TypeLayout {
        TypeLayout::basic(ByteUnits::of(4))
    }

    fn long_layout(&self) -> TypeLayout {
        if self.os().is_windows() {
            TypeLayout::basic(ByteUnits::of(4))
        } else {
            TypeLayout::basic(ByteUnits::of(8))
        }
    }

    fn longlong_layout(&self) -> TypeLayout {
        TypeLayout::basic(ByteUnits::of(8))
    }

    fn float_layout(&self) -> TypeLayout {
        TypeLayout::basic(ByteUnits::of(4))
    }

    fn double_layout(&self) -> TypeLayout {
        TypeLayout::basic(ByteUnits::of(8))
    }

    fn size_layout(&self) -> TypeLayout {
        TypeLayout::basic(ByteUnits::of(8))
    }

    fn c_integer_bytes(&self, c_integer: CInteger) -> ByteUnits {
        match c_integer {
            CInteger::Char => self.char_layout().width,
            CInteger::Short => self.short_layout().width,
            CInteger::Int => self.int_layout().width,
            CInteger::Long => self.long_layout().width,
            CInteger::LongLong => self.longlong_layout().width,
        }
    }
}
