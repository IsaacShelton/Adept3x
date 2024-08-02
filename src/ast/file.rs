use super::{
    enumeration::Enum, global_variable::Global, named_expr::Define, structure::Structure,
    type_alias::Alias, Function,
};
use crate::file_id::FileId;
use indexmap::IndexMap;

#[derive(Clone, Debug)]
pub struct AstFile {
    pub file_id: FileId,
    pub functions: Vec<Function>,
    pub structures: Vec<Structure>,
    pub aliases: IndexMap<String, Alias>,
    pub globals: Vec<Global>,
    pub enums: IndexMap<String, Enum>,
    pub defines: IndexMap<String, Define>,
}

impl AstFile {
    pub fn new(file_id: FileId) -> AstFile {
        AstFile {
            file_id,
            functions: vec![],
            structures: vec![],
            aliases: IndexMap::default(),
            globals: vec![],
            enums: IndexMap::default(),
            defines: IndexMap::default(),
        }
    }
}
