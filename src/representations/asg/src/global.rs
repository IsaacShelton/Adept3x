use super::{GlobalRef, Type};
use crate::name::ResolvedName;
use attributes::{Privacy, SymbolOwnership};
use source_files::Source;

#[derive(Clone, Debug)]
pub struct Global {
    pub name: ResolvedName,
    pub ty: Type,
    pub source: Source,
    pub is_thread_local: bool,
    pub ownership: SymbolOwnership,
}

#[derive(Clone, Debug)]
pub struct GlobalDecl {
    pub global_ref: GlobalRef,
    pub privacy: Privacy,
}
