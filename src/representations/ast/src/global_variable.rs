use crate::Type;
use attributes::{Privacy, SymbolOwnership};
use source_files::Source;

#[derive(Clone, Debug)]
pub struct Global {
    pub name: String,
    pub ast_type: Type,
    pub source: Source,
    pub is_thread_local: bool,
    pub privacy: Privacy,
    pub ownership: SymbolOwnership,
}
