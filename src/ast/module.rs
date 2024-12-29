use super::{
    enumeration::Enum, func::Func, global_variable::GlobalVar, structs::Struct,
    type_alias::TypeAlias, HelperExpr,
};
use crate::version::AdeptVersion;
use indexmap::IndexMap;

#[derive(Clone, Debug)]
pub struct AstModule {
    pub adept_version: AdeptVersion,
    pub functions: Vec<Func>,
    pub structures: Vec<Struct>,
    pub type_aliases: IndexMap<String, TypeAlias>,
    pub global_variables: Vec<GlobalVar>,
    pub enums: IndexMap<String, Enum>,
    pub helper_exprs: IndexMap<String, HelperExpr>,
}
