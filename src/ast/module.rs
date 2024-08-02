use super::{
    enumeration::Enum, function::Function, global_variable::GlobalVar, structure::Structure,
    type_alias::TypeAlias, HelperExpr,
};
use crate::version::AdeptVersion;
use indexmap::IndexMap;

#[derive(Clone, Debug)]
pub struct AstModule {
    pub adept_version: AdeptVersion,
    pub functions: Vec<Function>,
    pub structures: Vec<Structure>,
    pub type_aliases: IndexMap<String, TypeAlias>,
    pub global_variables: Vec<GlobalVar>,
    pub enums: IndexMap<String, Enum>,
    pub helper_exprs: IndexMap<String, HelperExpr>,
}
