use crate::{
    Enum, Expr, ExprAlias, Func, Global, Impl, Struct, Trait, TypeAlias, UseNamespace, When,
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
    pub whens: Vec<When>,
    pub pragmas: Vec<Expr>,
    pub use_namespaces: Vec<UseNamespace>,
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
