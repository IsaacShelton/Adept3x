/*
    ==================  components/job/src/typed_cfg/mod.rs  ==================
    Contains definitions for typing and resolving references for a CFG
    ---------------------------------------------------------------------------
*/

mod value;

use crate::repr::{Type, TypeKind};
use source_files::Source;
pub use value::*;

#[derive(Clone, Debug)]
pub struct Typed<'env> {
    ty: Type<'env>,
}

impl<'env> Typed<'env> {
    pub fn from_type(ty: Type<'env>) -> Self {
        Self { ty }
    }

    pub fn void(source: Source) -> Self {
        Self::from_type(Type {
            kind: TypeKind::Void,
            source,
        })
    }

    pub fn ty(&self) -> &Type<'env> {
        &self.ty
    }
}
