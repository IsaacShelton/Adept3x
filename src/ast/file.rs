use super::{
    enumeration::Enum, global_variable::GlobalVar, structure::Structure, type_alias::TypeAlias,
    Function, HelperExpr, SettingsId,
};
use indexmap::IndexMap;

#[derive(Clone, Debug)]
pub struct AstFile {
    pub functions: Vec<Function>,
    pub structures: Vec<Structure>,
    pub type_aliases: IndexMap<String, TypeAlias>,
    pub global_variables: Vec<GlobalVar>,
    pub enums: IndexMap<String, Enum>,
    pub helper_exprs: IndexMap<String, HelperExpr>,
    pub settings: Option<SettingsId>,
}

impl AstFile {
    pub fn new() -> AstFile {
        AstFile {
            functions: vec![],
            structures: vec![],
            type_aliases: IndexMap::default(),
            global_variables: vec![],
            enums: IndexMap::default(),
            helper_exprs: IndexMap::default(),
            settings: None,
        }
    }
}
