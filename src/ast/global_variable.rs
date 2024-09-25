use super::Type;
use crate::{name::Name, source_files::Source};

#[derive(Clone, Debug)]
pub struct GlobalVar {
    pub name: Name,
    pub ast_type: Type,
    pub source: Source,
    pub is_foreign: bool,
    pub is_thread_local: bool,
}
