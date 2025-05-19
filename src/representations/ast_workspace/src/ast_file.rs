use crate::NameScopeRef;
use ast::{Enum, ExprAlias, Func, Global, Impl, Struct, Trait, TypeAlias};
use ast_workspace_settings::{Settings, SettingsRef};

#[derive(Clone, Debug)]
pub struct AstFile {
    pub settings: Option<SettingsRef>,
    pub names: NameScopeRef,
}

#[derive(Debug)]
pub struct AstFileView<'workspace> {
    pub settings: Option<&'workspace Settings>,
    pub funcs: &'workspace [Func],
    pub structs: &'workspace [Struct],
    pub enums: &'workspace [Enum],
    pub globals: &'workspace [Global],
    pub type_aliases: &'workspace [TypeAlias],
    pub expr_aliases: &'workspace [ExprAlias],
    pub traits: &'workspace [Trait],
    pub impls: &'workspace [Impl],
}
