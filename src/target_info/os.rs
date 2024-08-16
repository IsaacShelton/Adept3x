#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TargetOs {
    Windows,
    Mac,
    Linux,
}

impl TargetOs {
    pub const HOST: Option<Self> = if cfg!(target_os = "windows") {
        Some(TargetOs::Windows)
    } else if cfg!(target_os = "macos") {
        Some(TargetOs::Mac)
    } else if cfg!(target_os = "linux") {
        Some(TargetOs::Linux)
    } else {
        None
    };
}

pub trait TargetOsExt {
    fn is_host(&self) -> bool;
    fn is_windows(&self) -> bool;
    fn is_mac(&self) -> bool;
    fn is_linux(&self) -> bool;
}

impl TargetOsExt for TargetOs {
    fn is_host(&self) -> bool {
        TargetOs::HOST.map_or(false, |os| *self == os)
    }

    fn is_windows(&self) -> bool {
        matches!(self, TargetOs::Windows)
    }

    fn is_mac(&self) -> bool {
        matches!(self, TargetOs::Mac)
    }

    fn is_linux(&self) -> bool {
        matches!(self, TargetOs::Linux)
    }
}

impl TargetOsExt for Option<TargetOs> {
    fn is_host(&self) -> bool {
        *self == TargetOs::HOST
    }

    fn is_windows(&self) -> bool {
        matches!(self, Some(TargetOs::Windows))
    }

    fn is_mac(&self) -> bool {
        matches!(self, Some(TargetOs::Mac))
    }

    fn is_linux(&self) -> bool {
        matches!(self, Some(TargetOs::Linux))
    }
}
