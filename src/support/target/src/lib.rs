mod arch;
mod display;
mod os;

pub use arch::{TargetArch, TargetArchExt};
pub use display::IntoDisplay;
pub use os::{TargetOs, TargetOsExt};
use primitives::{CInteger, FloatOrSign, FloatOrSignLax, IntegerSign, NumericMode};
use std::{
    ffi::{OsStr, OsString},
    fmt::Display,
};

#[derive(Copy, Clone, Debug, Default)]
pub struct Target {
    arch: Option<TargetArch>,
    os: Option<TargetOs>,
}

impl Target {
    pub const HOST: Self = Self::new(TargetOs::HOST, TargetArch::HOST);
    pub const SANDBOX: Self = Self::new(None, None);

    pub const fn new(os: Option<TargetOs>, arch: Option<TargetArch>) -> Self {
        Self { arch, os }
    }

    pub const fn generic_os(os: TargetOs) -> Self {
        let arch = match os {
            TargetOs::Windows | TargetOs::Mac | TargetOs::Linux | TargetOs::FreeBsd => {
                TargetArch::X86_64
            }
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

        match self.os() {
            Some(TargetOs::Windows) => {
                if self.arch.is_x86_64() {
                    format!("{}.exe", basename)
                } else {
                    format!("{}-{}.exe", basename, self.arch.display())
                }
            }
            Some(TargetOs::Mac | TargetOs::Linux | TargetOs::FreeBsd) | None => {
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
        // Everything except `char` is always signed by default.
        // On Darwin, `char` is also always signed by default.
        if integer != CInteger::Char || self.os.is_mac() {
            return IntegerSign::Signed;
        }

        // Otherwise, it's architecture-dependent.
        // What's the sign of `char` on this architecture?
        match &self.arch {
            Some(TargetArch::X86_64) => IntegerSign::Signed,
            None | Some(TargetArch::Aarch64) => IntegerSign::Unsigned,
        }
    }

    pub fn default_float_or_sign_from_lax(
        &self,
        float_or_sign_lax: &FloatOrSignLax,
    ) -> FloatOrSign {
        match float_or_sign_lax {
            FloatOrSignLax::Integer(sign) => FloatOrSign::Integer(*sign),
            FloatOrSignLax::IndeterminateInteger(c_integer) => {
                FloatOrSign::Integer(self.default_c_integer_sign(*c_integer))
            }
            FloatOrSignLax::Float => FloatOrSign::Float,
        }
    }

    pub fn default_float_or_sign(&self, numeric_mode: NumericMode) -> FloatOrSign {
        match numeric_mode {
            NumericMode::Integer(sign) => FloatOrSign::Integer(sign),
            NumericMode::LooseIndeterminateSignInteger(c_integer) => {
                FloatOrSign::Integer(self.default_c_integer_sign(c_integer))
            }
            NumericMode::CheckOverflow(_, sign) => FloatOrSign::Integer(sign),
            NumericMode::Float => FloatOrSign::Float,
        }
    }

    pub fn is_little_endian(&self) -> bool {
        match &self.arch {
            None | Some(TargetArch::X86_64 | TargetArch::Aarch64) => true,
        }
    }
}

impl Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {}",
            self.arch.map_or("bytecode".into(), |arch| arch.to_string()),
            self.os.map_or("sandbox".into(), |os| os.to_string())
        )
    }
}
