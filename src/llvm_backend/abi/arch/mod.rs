pub mod aarch64;
pub mod x86_64;

use self::{aarch64::AARCH64, x86_64::X86_64};
use crate::target_info::{type_layout::TypeLayoutCache, TargetInfo};

#[derive(Clone, Debug)]
pub enum Arch<'a> {
    X86_64(X86_64),
    AARCH64(AARCH64<'a>),
}

pub struct CoreInfo<'a> {
    pub type_layout_cache: &'a TypeLayoutCache<'a>,
    pub target_info: &'a TargetInfo,
}

impl<'a> Arch<'a> {
    pub fn core_info(&self) -> CoreInfo<'a> {
        match self {
            Arch::X86_64(_arch) => todo!(),
            Arch::AARCH64(arch) => CoreInfo {
                type_layout_cache: &arch.type_layout_cache,
                target_info: &arch.target_info,
            },
        }
    }
}
