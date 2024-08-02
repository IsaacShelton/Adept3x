pub mod record_layout;
pub mod type_layout;

use crate::{
    ast::{CInteger, IntegerSign},
    data_units::ByteUnits,
};
use derive_more::IsVariant;
use type_layout::TypeLayout;

#[derive(Clone, Debug)]
pub struct TargetInfo {
    pub kind: TargetInfoKind,
    pub ms_abi: bool,
    pub is_darwin: bool,
}

impl TargetInfo {
    pub fn arbitrary() -> Self {
        Self {
            kind: TargetInfoKind::Arbitrary,
            ms_abi: false,
            is_darwin: false,
        }
    }
}

#[derive(Clone, Debug, IsVariant)]
pub enum TargetInfoKind {
    Arbitrary,
    X86_64,
    AARCH64,
}

impl TargetInfo {
    pub fn default_c_integer_sign(&self, integer: CInteger) -> IntegerSign {
        // Non-`char` integer types are signed by default.
        // On darwin, `char` is also always signed.
        if integer != CInteger::Char || self.is_darwin {
            return IntegerSign::Signed;
        }

        // Otherwise, the signness of `char` depends on the architecture
        match &self.kind {
            TargetInfoKind::Arbitrary => IntegerSign::Unsigned,
            TargetInfoKind::X86_64 => IntegerSign::Signed,
            TargetInfoKind::AARCH64 => IntegerSign::Unsigned,
        }
    }

    pub fn is_little_endian(&self) -> bool {
        match &self.kind {
            TargetInfoKind::Arbitrary | TargetInfoKind::X86_64 | TargetInfoKind::AARCH64 => true,
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
        if self.ms_abi {
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
