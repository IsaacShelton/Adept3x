use crate::target_info::{TargetArch, TargetOs};
pub mod aarch64;
pub mod x86_64;

use self::{aarch64::Aarch64, x86_64::X86_64};
use crate::{ir, target_info::TargetInfo};
use aarch64::Aarch64Variant;
use derive_more::IsVariant;
use x86_64::{AvxLevel, SysV, Win64};

#[derive(Clone, Debug, IsVariant)]
pub enum Arch {
    X86_64(X86_64),
    Aarch64(Aarch64),
}

impl Arch {
    pub fn new(target: &TargetInfo) -> Option<Self> {
        match target.arch {
            Some(TargetArch::X86_64) => {
                let avx_level = AvxLevel::None;

                Some(Arch::X86_64(match target.os {
                    Some(TargetOs::Mac | TargetOs::Linux) => X86_64::SysV(SysV {
                        os: target.os.as_ref().try_into().ok()?,
                        avx_level,
                    }),
                    Some(TargetOs::Windows) => X86_64::Win64(Win64 {
                        is_mingw: true,
                        avx_level,
                    }),
                    None => todo!(),
                }))
            }
            Some(TargetArch::Aarch64) => Some(Arch::Aarch64(Aarch64 {
                variant: match target.os {
                    Some(TargetOs::Windows) => Aarch64Variant::Win64,
                    Some(TargetOs::Mac) => Aarch64Variant::DarwinPCS,
                    Some(TargetOs::Linux) => Aarch64Variant::Aapcs,
                    None => return None,
                },
                is_cxx_mode: false,
            })),
            None => None,
        }
    }
}

pub fn use_first_field_if_transparent_union(ty: &ir::Type) -> &ir::Type {
    // NOTE: We don't support transparent unions yet
    ty
}
