use super::{
    enumeration::Enum, function::Function, global_variable::Global, named_expr::Define,
    structure::Structure, type_alias::Alias,
};
use crate::version::AdeptVersion;
use indexmap::IndexMap;

#[derive(Clone, Debug)]
pub struct AstModule {
    pub adept_version: AdeptVersion,
    pub functions: Vec<Function>,
    pub structures: Vec<Structure>,
    pub aliases: IndexMap<String, Alias>,
    pub globals: Vec<Global>,
    pub enums: IndexMap<String, Enum>,
    pub defines: IndexMap<String, Define>,
}
