pub mod aarch64;
pub mod x86_64;

use self::{aarch64::Aarch64, x86_64::X86_64};
use crate::ir;
use derive_more::IsVariant;

#[derive(Clone, Debug, IsVariant)]
pub enum Arch {
    X86_64(X86_64),
    Aarch64(Aarch64),
}

pub fn use_first_field_if_transparent_union(ty: &ir::Type) -> &ir::Type {
    // NOTE: We don't support transparent unions yet
    ty
}
