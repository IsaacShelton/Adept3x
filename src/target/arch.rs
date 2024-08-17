use std::fmt::Display;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TargetArch {
    X86_64,
    Aarch64,
}

impl TargetArch {
    pub const HOST: Option<Self> = if cfg!(target_arch = "x86_64") || cfg!(target_os = "windows") {
        // NOTE: We always consider x86_64 to be host architecture on Windows for portability
        Some(TargetArch::X86_64)
    } else if cfg!(target_arch = "aarch64") {
        Some(TargetArch::Aarch64)
    } else {
        None
    };
}

impl Display for TargetArch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TargetArch::X86_64 => write!(f, "x86_64"),
            TargetArch::Aarch64 => write!(f, "aarch64"),
        }
    }
}

pub trait TargetArchExt {
    fn is_host(&self) -> bool;
    fn is_x86_64(&self) -> bool;
    fn is_aarch64(&self) -> bool;
}

impl TargetArchExt for TargetArch {
    fn is_host(&self) -> bool {
        TargetArch::HOST.map_or(false, |arch| *self == arch)
    }

    fn is_x86_64(&self) -> bool {
        matches!(self, TargetArch::X86_64)
    }

    fn is_aarch64(&self) -> bool {
        matches!(self, TargetArch::Aarch64)
    }
}

impl TargetArchExt for Option<TargetArch> {
    fn is_host(&self) -> bool {
        *self == TargetArch::HOST
    }

    fn is_x86_64(&self) -> bool {
        matches!(self, Some(TargetArch::X86_64))
    }

    fn is_aarch64(&self) -> bool {
        matches!(self, Some(TargetArch::Aarch64))
    }
}
