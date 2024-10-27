use super::TypeKind;
use crate::{ast::Privacy, source_files::Source};

#[derive(Clone, Debug)]
pub struct TypeDecl {
    pub kind: TypeKind,
    pub source: Source,
    pub privacy: Privacy,
}
