mod aarch64;
mod x86_64;

pub use self::{aarch64::AARCH64, x86_64::X86_64};

#[derive(Clone, Debug)]
pub enum Arch {
    X86_64(X86_64),
    AARCH64(AARCH64),
}
