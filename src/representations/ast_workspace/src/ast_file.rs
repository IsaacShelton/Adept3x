use crate::NameScopeRef;
use ast::{Enum, ExprAlias, Func, Global, Impl, Struct, Trait, TypeAlias};
use ast_workspace_settings::{Settings, SettingsRef};

#[derive(Copy, Clone, Debug)]
pub struct AstFile {
    pub settings: SettingsRef,
    pub names: NameScopeRef,
}

#[derive(Debug)]
pub struct AstFileView<'workspace> {
    pub settings: &'workspace Settings,
    pub funcs: Vec<&'workspace Func>,
    pub structs: Vec<&'workspace Struct>,
    pub enums: Vec<&'workspace Enum>,
    pub globals: Vec<&'workspace Global>,
    pub type_aliases: Vec<&'workspace TypeAlias>,
    pub expr_aliases: Vec<&'workspace ExprAlias>,
    pub traits: Vec<&'workspace Trait>,
    pub impls: Vec<&'workspace Impl>,
}
