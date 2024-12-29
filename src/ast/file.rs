use super::{
    enumeration::Enum, global_variable::GlobalVar, implementation::Impl, structs::Struct,
    type_alias::TypeAlias, Func, HelperExpr, SettingsId, Trait,
};

#[derive(Clone, Debug)]
pub struct AstFile {
    pub funcs: Vec<Func>,
    pub structures: Vec<Struct>,
    pub type_aliases: Vec<TypeAlias>,
    pub global_variables: Vec<GlobalVar>,
    pub enums: Vec<Enum>,
    pub helper_exprs: Vec<HelperExpr>,
    pub traits: Vec<Trait>,
    pub impls: Vec<Impl>,
    pub settings: Option<SettingsId>,
}

impl AstFile {
    pub fn new() -> AstFile {
        AstFile {
            funcs: vec![],
            structures: vec![],
            type_aliases: vec![],
            global_variables: vec![],
            enums: vec![],
            helper_exprs: vec![],
            traits: vec![],
            impls: vec![],
            settings: None,
        }
    }
}
