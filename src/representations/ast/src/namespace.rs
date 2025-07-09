use crate::{
    ConditionalCompilation, Enum, ExprAlias, Func, Global, Impl, Struct, Trait, TypeAlias,
};
use attributes::Privacy;
use source_files::Source;

#[derive(Clone, Debug, Default)]
pub struct NamespaceItems {
    pub funcs: Vec<Func>,
    pub structs: Vec<Struct>,
    pub enums: Vec<Enum>,
    pub globals: Vec<Global>,
    pub type_aliases: Vec<TypeAlias>,
    pub expr_aliases: Vec<ExprAlias>,
    pub traits: Vec<Trait>,
    pub impls: Vec<Impl>,
    pub namespaces: Vec<Namespace>,
    pub conditional_compilations: Vec<ConditionalCompilation>,
}

#[derive(Clone, Debug)]
pub struct Namespace {
    pub name: String,
    pub items: NamespaceItems,
    pub source: Source,
    pub privacy: Privacy,
}
