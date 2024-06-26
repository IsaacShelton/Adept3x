pub mod aarch64;
pub mod x86_64;

use self::{aarch64::AARCH64, x86_64::X86_64};
use crate::target_info::{type_info::TypeInfoManager, TargetInfo};

#[derive(Clone, Debug)]
pub enum Arch<'a> {
    X86_64(X86_64),
    AARCH64(AARCH64<'a>),
}

pub struct CoreInfo<'a> {
    pub type_info_manager: &'a TypeInfoManager,
    pub target_info: &'a TargetInfo,
}

impl<'a> Arch<'a> {
    pub fn core_info(&self) -> CoreInfo<'a> {
        match self {
            Arch::X86_64(_arch) => todo!(),
            Arch::AARCH64(arch) => CoreInfo {
                type_info_manager: &arch.type_info_manager,
                target_info: &arch.target_info,
            },
        }
    }
}
