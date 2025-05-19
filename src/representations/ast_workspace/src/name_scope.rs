use crate::{
    EnumId, ExprAliasId, FuncId, GlobalId, ImplId, NameScopeRef, Namespace, NamespaceId, StructId,
    TraitId, TypeAliasId, TypeDeclRef,
};
use arena::IdxSpan;
use ast::{Enum, ExprAlias, Func, Global, Impl, Struct, Trait, TypeAlias};

#[derive(Clone, Debug)]
pub struct NameScope {
    pub funcs: IdxSpan<FuncId, Func>,
    pub structs: IdxSpan<StructId, Struct>,
    pub enums: IdxSpan<EnumId, Enum>,
    pub globals: IdxSpan<GlobalId, Global>,
    pub type_aliases: IdxSpan<TypeAliasId, TypeAlias>,
    pub expr_aliases: IdxSpan<ExprAliasId, ExprAlias>,
    pub traits: IdxSpan<TraitId, Trait>,
    pub impls: IdxSpan<ImplId, Impl>,
    pub namespaces: IdxSpan<NamespaceId, Namespace>,
    pub parent: Option<NameScopeRef>,
}

impl NameScope {
    pub fn direct_type_decls(&self) -> impl Iterator<Item = TypeDeclRef> {
        self.structs
            .iter()
            .map(TypeDeclRef::Struct)
            .chain(self.enums.iter().map(TypeDeclRef::Enum))
            .chain(self.type_aliases.iter().map(TypeDeclRef::Alias))
            .chain(self.traits.iter().map(TypeDeclRef::Trait))
    }
}
