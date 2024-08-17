use crate::target::{TargetArch, TargetOs};
pub mod aarch64;
pub mod x86_64;

use self::{aarch64::Aarch64, x86_64::X86_64};
use crate::{ir, target::Target};
use aarch64::Aarch64Variant;
use derive_more::IsVariant;
use x86_64::{AvxLevel, SysV, SysVOs, Win64};

#[derive(Clone, Debug, IsVariant)]
pub enum Arch {
    X86_64(X86_64),
    Aarch64(Aarch64),
}

impl Arch {
    pub fn new(target: &Target) -> Option<Self> {
        Some(match target.arch().as_ref()? {
            TargetArch::X86_64 => {
                let avx_level = AvxLevel::None;

                Arch::X86_64(match target.os()? {
                    TargetOs::Windows => X86_64::Win64(Win64::new(avx_level)),
                    TargetOs::Mac => X86_64::SysV(SysV::new(SysVOs::Darwin, avx_level)),
                    TargetOs::Linux => X86_64::SysV(SysV::new(SysVOs::Linux, avx_level)),
                })
            }
            TargetArch::Aarch64 => Arch::Aarch64(Aarch64 {
                variant: match target.os()? {
                    TargetOs::Windows => Aarch64Variant::Win64,
                    TargetOs::Mac => Aarch64Variant::DarwinPCS,
                    TargetOs::Linux => Aarch64Variant::Aapcs,
                },
                is_cxx_mode: false,
            }),
        })
    }
}

pub fn use_first_field_if_transparent_union(ty: &ir::Type) -> &ir::Type {
    // NOTE: We don't support transparent unions yet
    ty
}
