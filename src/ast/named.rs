use super::{enumeration::Enum, named_expr::Define, type_alias::Alias};

#[derive(Clone, Debug)]
pub struct NamedAlias {
    pub name: String,
    pub alias: Alias,
}

#[derive(Clone, Debug)]
pub struct NamedEnum {
    pub name: String,
    pub enum_definition: Enum,
}

#[derive(Clone, Debug)]
pub struct NamedDefine {
    pub name: String,
    pub define: Define,
}
