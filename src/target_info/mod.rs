mod arch;
mod os;
pub mod record_layout;
pub mod type_layout;

use crate::{
    ast::{CInteger, IntegerSign},
    data_units::ByteUnits,
};
pub use arch::TargetArch;
use arch::TargetArchExt;
pub use os::{TargetOs, TargetOsExt};
use type_layout::TypeLayout;

#[derive(Clone, Debug, Default)]
pub struct TargetInfo {
    pub arch: Option<TargetArch>,
    pub os: Option<TargetOs>,
}

impl TargetInfo {
    pub fn is_host(&self) -> bool {
        self.arch.is_host() && self.os.is_host()
    }

    pub fn default_c_integer_sign(&self, integer: CInteger) -> IntegerSign {
        // Non-`char` integer types are signed by default.
        // On darwin, `char` is also always signed.
        if integer != CInteger::Char || self.os.is_mac() {
            return IntegerSign::Signed;
        }

        // Otherwise, the signness of `char` depends on the architecture
        match &self.arch {
            None => IntegerSign::Unsigned,
            Some(TargetArch::X86_64) => IntegerSign::Signed,
            Some(TargetArch::Aarch64) => IntegerSign::Unsigned,
        }
    }

    pub fn is_little_endian(&self) -> bool {
        match &self.arch {
            None | Some(TargetArch::X86_64) | Some(TargetArch::Aarch64) => true,
        }
    }

    pub fn pointer_layout(&self) -> TypeLayout {
        TypeLayout::basic(ByteUnits::of(8))
    }

    pub fn bool_layout(&self) -> TypeLayout {
        TypeLayout::basic(ByteUnits::of(1))
    }

    pub fn char_layout(&self) -> TypeLayout {
        TypeLayout::basic(ByteUnits::of(1))
    }

    pub fn short_layout(&self) -> TypeLayout {
        TypeLayout::basic(ByteUnits::of(2))
    }

    pub fn int_layout(&self) -> TypeLayout {
        TypeLayout::basic(ByteUnits::of(4))
    }

    pub fn long_layout(&self) -> TypeLayout {
        if self.os.is_windows() {
            TypeLayout::basic(ByteUnits::of(4))
        } else {
            TypeLayout::basic(ByteUnits::of(8))
        }
    }

    pub fn longlong_layout(&self) -> TypeLayout {
        TypeLayout::basic(ByteUnits::of(8))
    }

    pub fn float_layout(&self) -> TypeLayout {
        TypeLayout::basic(ByteUnits::of(4))
    }

    pub fn double_layout(&self) -> TypeLayout {
        TypeLayout::basic(ByteUnits::of(8))
    }
}
