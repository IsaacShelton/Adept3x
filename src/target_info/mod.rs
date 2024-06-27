pub mod record_layout;
pub mod type_info;

use crate::ast::{CInteger, IntegerSign};
use type_info::TypeInfo;

#[derive(Clone, Debug)]
pub struct TargetInfo {
    pub kind: TargetInfoKind,
    pub ms_abi: bool,
    pub is_darwin: bool,
}

#[derive(Clone, Debug)]
pub enum TargetInfoKind {
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
            TargetInfoKind::X86_64 => IntegerSign::Signed,
            TargetInfoKind::AARCH64 => IntegerSign::Unsigned,
        }
    }

    pub fn is_little_endian(&self) -> bool {
        match &self.kind {
            TargetInfoKind::X86_64 | TargetInfoKind::AARCH64 => true,
        }
    }

    pub fn pointer_layout(&self) -> TypeInfo {
        TypeInfo::basic(8)
    }

    pub fn bool_layout(&self) -> TypeInfo {
        TypeInfo::basic(1)
    }

    pub fn char_layout(&self) -> TypeInfo {
        TypeInfo::basic(1)
    }

    pub fn short_layout(&self) -> TypeInfo {
        TypeInfo::basic(2)
    }

    pub fn long_layout(&self) -> TypeInfo {
        if self.ms_abi {
            TypeInfo::basic(4)
        } else {
            TypeInfo::basic(8)
        }
    }

    pub fn longlong_layout(&self) -> TypeInfo {
        TypeInfo::basic(8)
    }

    pub fn float_layout(&self) -> TypeInfo {
        TypeInfo::basic(4)
    }

    pub fn double_layout(&self) -> TypeInfo {
        TypeInfo::basic(8)
    }
}
