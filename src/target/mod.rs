mod arch;
mod display;
mod os;
pub mod record_layout;
pub mod type_layout;

use crate::{
    ast::{CInteger, IntegerSign},
    data_units::ByteUnits,
};
pub use arch::{TargetArch, TargetArchExt};
pub use display::IntoDisplay;
pub use os::{TargetOs, TargetOsExt};
use std::{
    ffi::{OsStr, OsString},
    fmt::Display,
};
use type_layout::TypeLayout;

#[derive(Copy, Clone, Debug, Default)]
pub struct Target {
    arch: Option<TargetArch>,
    os: Option<TargetOs>,
}

impl Target {
    pub const HOST: Self = Self::new(TargetOs::HOST, TargetArch::HOST);

    pub const fn new(os: Option<TargetOs>, arch: Option<TargetArch>) -> Self {
        Self { arch, os }
    }

    pub const fn generic_os(os: TargetOs) -> Self {
        let arch = match os {
            TargetOs::Windows | TargetOs::Mac | TargetOs::Linux => TargetArch::X86_64,
        };

        Self::new(Some(os), Some(arch))
    }

    pub fn os(&self) -> Option<TargetOs> {
        self.os
    }

    pub fn arch(&self) -> Option<TargetArch> {
        self.arch
    }

    pub fn is_host(&self) -> bool {
        self.arch.is_host() && self.os.is_host()
    }

    pub fn default_executable_name(&self, basename: &OsStr) -> OsString {
        let basename = basename.to_str().unwrap_or("main");

        match self.os {
            Some(TargetOs::Windows) => {
                format!(
                    "{}-{}-{}.exe",
                    basename,
                    self.arch.display(),
                    self.os.display()
                )
            }
            Some(TargetOs::Mac | TargetOs::Linux) | None => {
                format!("{}-{}-{}", basename, self.arch.display(), self.os.display())
            }
        }
        .into()
    }

    pub fn default_object_file_name(&self, basename: &OsStr) -> OsString {
        format!(
            "{}-{}-{}.o",
            basename.to_str().unwrap_or("main"),
            self.arch.display(),
            self.os.display()
        )
        .into()
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

impl Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {}",
            self.arch.map_or("unknown".into(), |arch| arch.to_string()),
            self.os.map_or("unknown".into(), |os| os.to_string())
        )
    }
}
