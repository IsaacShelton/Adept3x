use crate::{
    EnumId, ExprAliasId, FuncId, GlobalId, ImplId, NameScope, NameScopeId, NameScopeRef, Namespace,
    NamespaceId, StructId, TraitId, TypeAliasId,
};
use arena::{Arena, IdxSpan};
use ast::{Enum, ExprAlias, Func, Global, Impl, NamespaceItems, Struct, Trait, TypeAlias};

#[derive(Debug, Default)]
pub struct AstWorkspaceSymbols {
    pub all_funcs: Arena<FuncId, Func>,
    pub all_structs: Arena<StructId, Struct>,
    pub all_enums: Arena<EnumId, Enum>,
    pub all_globals: Arena<GlobalId, Global>,
    pub all_type_aliases: Arena<TypeAliasId, TypeAlias>,
    pub all_expr_aliases: Arena<ExprAliasId, ExprAlias>,
    pub all_traits: Arena<TraitId, Trait>,
    pub all_impls: Arena<ImplId, Impl>,
    pub all_namespaces: Arena<NamespaceId, Namespace>,
    pub all_name_scopes: Arena<NameScopeId, NameScope>,
}

impl AstWorkspaceSymbols {
    pub fn new_name_scope(
        &mut self,
        items: NamespaceItems,
        parent: Option<NameScopeRef>,
    ) -> NameScopeRef {
        let funcs = self.all_funcs.alloc_many(items.funcs);
        let structs = self.all_structs.alloc_many(items.structs);
        let enums = self.all_enums.alloc_many(items.enums);
        let globals = self.all_globals.alloc_many(items.globals);
        let type_aliases = self.all_type_aliases.alloc_many(items.type_aliases);
        let expr_aliases = self.all_expr_aliases.alloc_many(items.expr_aliases);
        let traits = self.all_traits.alloc_many(items.traits);
        let impls = self.all_impls.alloc_many(items.impls);

        let new_name_scope = self.all_name_scopes.alloc(NameScope {
            funcs,
            structs,
            enums,
            globals,
            type_aliases,
            expr_aliases,
            traits,
            impls,
            namespaces: IdxSpan::default(),
            parent,
        });

        let mut namespaces = Vec::with_capacity(items.namespaces.len());
        for namespace in items.namespaces {
            namespaces.push(Namespace {
                name: namespace.name,
                names: self.new_name_scope(namespace.items, Some(new_name_scope)),
                privacy: namespace.privacy,
            });
        }

        self.all_name_scopes[new_name_scope].namespaces =
            self.all_namespaces.alloc_many(namespaces.into_iter());
        new_name_scope
    }
}
