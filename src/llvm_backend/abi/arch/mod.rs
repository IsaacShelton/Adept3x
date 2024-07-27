pub mod aarch64;
pub mod x86_64;

use self::{aarch64::AARCH64, x86_64::X86_64};

#[derive(Clone, Debug)]
pub enum Arch {
    X86_64(X86_64),
    AARCH64(AARCH64),
}
