use super::{Executable, Execution, Spawnable};
use crate::{
    Continuation, Executor, TaskRef,
    repr::{Decl, DeclScope, DeclScopeOrigin, TypeRef},
};
use ast_workspace::AstWorkspace;
use by_address::ByAddress;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EstimateDeclScope<'env> {
    pub workspace: ByAddress<&'env AstWorkspace<'env>>,
    pub scope_origin: DeclScopeOrigin,
}

impl<'env> Executable<'env> for EstimateDeclScope<'env> {
    type Output = DeclScope;

    fn execute(self, _executor: &Executor<'env>) -> Result<Self::Output, Continuation<'env>> {
        let workspace = self.workspace;
        let mut scope = DeclScope::new();

        for name_scope in self
            .scope_origin
            .name_scopes(&workspace)
            .into_iter()
            .map(|name_scope_ref| &workspace.symbols.all_name_scopes[name_scope_ref])
        {
            for func_id in name_scope.funcs.iter() {
                let name = &workspace.symbols.all_funcs[func_id].head.name;
                scope.push_unique(name.into(), Decl::Func(func_id));
            }

            for impl_id in name_scope.impls.iter() {
                if let Some(name) = workspace.symbols.all_impls[impl_id].name.as_ref() {
                    scope.push_unique(name.into(), Decl::Impl(impl_id));
                }
            }

            for trait_id in name_scope.traits.iter() {
                let name = &workspace.symbols.all_traits[trait_id].name;
                scope.push_unique(name.into(), Decl::Type(TypeRef::Trait(trait_id)));
            }

            for struct_id in name_scope.structs.iter() {
                let name = &workspace.symbols.all_structs[struct_id].name;
                scope.push_unique(name.into(), Decl::Type(TypeRef::Struct(struct_id)));
            }

            for enum_id in name_scope.enums.iter() {
                let name = &workspace.symbols.all_enums[enum_id].name;
                scope.push_unique(name.into(), Decl::Type(TypeRef::Enum(enum_id)));
            }

            for type_alias_id in name_scope.type_aliases.iter() {
                let name = &workspace.symbols.all_type_aliases[type_alias_id].name;
                scope.push_unique(name.into(), Decl::Type(TypeRef::Alias(type_alias_id)));
            }

            for global_id in name_scope.globals.iter() {
                let name = &workspace.symbols.all_globals[global_id].name;
                scope.push_unique(name.into(), Decl::Global(global_id));
            }

            for expr_alias_id in name_scope.expr_aliases.iter() {
                let name = &workspace.symbols.all_expr_aliases[expr_alias_id].name;
                scope.push_unique(name.into(), Decl::ExprAlias(expr_alias_id));
            }

            for namespace_id in name_scope.namespaces.iter() {
                let namespace = &workspace.symbols.all_namespaces[namespace_id];
                scope.push_unique(namespace.name.clone(), Decl::Namespace(namespace_id));
            }
        }

        Ok(scope)
    }
}

impl<'env> Spawnable<'env> for EstimateDeclScope<'env> {
    fn spawn(&self) -> (Vec<TaskRef<'env>>, Execution<'env>) {
        (vec![], self.clone().into())
    }
}
