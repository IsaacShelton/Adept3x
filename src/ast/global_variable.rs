use super::Type;
use crate::source_files::Source;

#[derive(Clone, Debug)]
pub struct GlobalVar {
    pub name: String,
    pub ast_type: Type,
    pub source: Source,
    pub is_foreign: bool,
    pub is_thread_local: bool,
}
