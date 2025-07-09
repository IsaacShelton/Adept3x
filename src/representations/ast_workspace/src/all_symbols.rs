use crate::{
    ConditionalNameScopeId, EnumId, ExprAliasId, FuncId, GlobalId, ImplId, NameScope, NameScopeId,
    NameScopeRef, Namespace, NamespaceId, StructId, TraitId, TypeAliasId,
    conditional_name_scope::ConditionalNameScope,
};
use arena::{IdxSpan, LockFreeArena};
use ast::{Enum, ExprAlias, Func, Global, Impl, NamespaceItems, Struct, Trait, TypeAlias};
use ast_workspace_settings::SettingsRef;

#[derive(Debug, Default)]
pub struct AstWorkspaceSymbols {
    pub all_funcs: LockFreeArena<FuncId, Func>,
    pub all_structs: LockFreeArena<StructId, Struct>,
    pub all_enums: LockFreeArena<EnumId, Enum>,
    pub all_globals: LockFreeArena<GlobalId, Global>,
    pub all_type_aliases: LockFreeArena<TypeAliasId, TypeAlias>,
    pub all_expr_aliases: LockFreeArena<ExprAliasId, ExprAlias>,
    pub all_traits: LockFreeArena<TraitId, Trait>,
    pub all_impls: LockFreeArena<ImplId, Impl>,
    pub all_namespaces: LockFreeArena<NamespaceId, Namespace>,
    pub all_name_scopes: LockFreeArena<NameScopeId, NameScope>,
    pub all_conditional_name_scopes: LockFreeArena<ConditionalNameScopeId, ConditionalNameScope>,
}

impl AstWorkspaceSymbols {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_name_scope(
        &mut self,
        items: NamespaceItems,
        parent: Option<NameScopeRef>,
        settings: SettingsRef,
    ) -> NameScopeRef {
        let funcs = self
            .all_funcs
            .alloc_many(items.funcs.into_iter().map(|f| Func {
                settings: Some(settings),
                ..f
            }));
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
            conditonal_name_scopes: IdxSpan::default(),
            parent,
        });

        let mut namespaces = Vec::with_capacity(items.namespaces.len());
        for namespace in items.namespaces {
            namespaces.push(Namespace {
                name: namespace.name,
                names: self.new_name_scope(namespace.items, Some(new_name_scope), settings),
                privacy: namespace.privacy,
            });
        }

        let mut conditional_name_scopes = Vec::with_capacity(items.conditional_compilations.len());
        for conditional in items.conditional_compilations {
            let conditions = conditional
                .conditions
                .into_iter()
                .map(|(expr, items)| {
                    (
                        expr,
                        self.new_name_scope(items, Some(new_name_scope), settings),
                    )
                })
                .collect();

            let otherwise = conditional
                .otherwise
                .map(|items| self.new_name_scope(items, Some(new_name_scope), settings));

            conditional_name_scopes.push(ConditionalNameScope {
                conditions,
                otherwise,
            });
        }

        self.all_name_scopes[new_name_scope].namespaces =
            self.all_namespaces.alloc_many(namespaces.into_iter());
        self.all_name_scopes[new_name_scope].conditonal_name_scopes = self
            .all_conditional_name_scopes
            .alloc_many(conditional_name_scopes.into_iter());
        new_name_scope
    }
}
