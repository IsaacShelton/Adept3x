use crate::{
    ConditionalCompilation, Enum, Expr, ExprAlias, Func, Global, Impl, Struct, Trait, TypeAlias,
};
use attributes::Privacy;
use derive_more::From;
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
    pub name: Option<String>,
    pub items: NamespaceItemsSource,
    pub source: Source,
    pub privacy: Option<Privacy>,
}

#[derive(Clone, Debug, From)]
pub enum NamespaceItemsSource {
    Items(NamespaceItems),
    Expr(Expr),
}
