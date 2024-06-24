pub mod type_info;

use type_info::TypeInfo;
use crate::ast::{CInteger, IntegerSign};

#[derive(Clone, Debug)]
pub struct TargetInfo {
    pub kind: TargetInfoKind,
    pub msabi: bool,
}

#[derive(Clone, Debug)]
pub enum TargetInfoKind {
    X86_64,
    AARCH64,
}

impl TargetInfo {
    pub fn default_c_integer_sign(&self, _integer: CInteger) -> IntegerSign {
        // For the platforms we support so far, all integers are signed by default
        IntegerSign::Signed
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
        if self.msabi {
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
