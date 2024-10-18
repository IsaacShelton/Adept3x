use super::{
    enumeration::Enum, global_variable::GlobalVar, structure::Structure, type_alias::TypeAlias,
    Function, HelperExpr, SettingsId,
};

#[derive(Clone, Debug)]
pub struct AstFile {
    pub functions: Vec<Function>,
    pub structures: Vec<Structure>,
    pub type_aliases: Vec<TypeAlias>,
    pub global_variables: Vec<GlobalVar>,
    pub enums: Vec<Enum>,
    pub helper_exprs: Vec<HelperExpr>,
    pub settings: Option<SettingsId>,
}

impl AstFile {
    pub fn new() -> AstFile {
        AstFile {
            functions: vec![],
            structures: vec![],
            type_aliases: vec![],
            global_variables: vec![],
            enums: vec![],
            helper_exprs: vec![],
            settings: None,
        }
    }
}
