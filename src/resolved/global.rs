use super::{GlobalVarRef, Type};
use crate::{ast::Privacy, name::ResolvedName, source_files::Source};

#[derive(Clone, Debug)]
pub struct GlobalVar {
    pub name: ResolvedName,
    pub resolved_type: Type,
    pub source: Source,
    pub is_foreign: bool,
    pub is_thread_local: bool,
}

#[derive(Clone, Debug)]
pub struct GlobalVarDecl {
    pub global_ref: GlobalVarRef,
    pub privacy: Privacy,
}
