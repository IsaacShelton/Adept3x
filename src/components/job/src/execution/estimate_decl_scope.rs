use super::Executable;
use crate::{
    Continuation, ExecutionCtx, Executor,
    repr::{Decl, DeclScope, DeclScopeOrigin},
};
use ast_workspace::{AstWorkspace, TypeDeclRef};
use by_address::ByAddress;
use derivative::Derivative;

#[derive(Clone, Derivative, PartialEq, Eq, Hash)]
#[derivative(Debug)]
pub struct EstimateDeclScope<'env> {
    pub scope_origin: DeclScopeOrigin,

    #[derivative(Debug = "ignore")]
    pub workspace: ByAddress<&'env AstWorkspace<'env>>,
}

impl<'env> EstimateDeclScope<'env> {
    pub fn new(workspace: &'env AstWorkspace<'env>, scope_origin: DeclScopeOrigin) -> Self {
        Self {
            workspace: ByAddress(workspace),
            scope_origin,
        }
    }
}

impl<'env> Executable<'env> for EstimateDeclScope<'env> {
    type Output = &'env DeclScope;

    fn execute(
        self,
        _executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
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
                scope.push_unique(name.into(), Decl::FuncLike(func_id));
            }

            for impl_id in name_scope.impls.iter() {
                if let Some(name) = workspace.symbols.all_impls[impl_id].name.as_ref() {
                    scope.push_unique(name.into(), Decl::TypeLike(impl_id.into()));
                }
            }

            for trait_id in name_scope.traits.iter() {
                let name = &workspace.symbols.all_traits[trait_id].name;
                scope.push_unique(
                    name.into(),
                    Decl::TypeLike(TypeDeclRef::Trait(trait_id).into()),
                );
            }

            for struct_id in name_scope.structs.iter() {
                let name = &workspace.symbols.all_structs[struct_id].name;
                scope.push_unique(
                    name.into(),
                    Decl::TypeLike(TypeDeclRef::Struct(struct_id).into()),
                );
            }

            for enum_id in name_scope.enums.iter() {
                let name = &workspace.symbols.all_enums[enum_id].name;
                scope.push_unique(
                    name.into(),
                    Decl::TypeLike(TypeDeclRef::Enum(enum_id).into()),
                );
            }

            for type_alias_id in name_scope.type_aliases.iter() {
                let name = &workspace.symbols.all_type_aliases[type_alias_id].name;
                scope.push_unique(
                    name.into(),
                    Decl::TypeLike(TypeDeclRef::Alias(type_alias_id).into()),
                );
            }

            for global_id in name_scope.globals.iter() {
                let name = &workspace.symbols.all_globals[global_id].name;
                scope.push_unique(name.into(), Decl::ValueLike(global_id.into()));
            }

            for expr_alias_id in name_scope.expr_aliases.iter() {
                let name = &workspace.symbols.all_expr_aliases[expr_alias_id].name;
                scope.push_unique(name.into(), Decl::ValueLike(expr_alias_id.into()));
            }

            for namespace_id in name_scope.namespaces.iter() {
                let namespace = &workspace.symbols.all_namespaces[namespace_id];
                scope.push_unique(namespace.name.clone(), Decl::TypeLike(namespace_id.into()));
            }
        }

        Ok(ctx.alloc(scope))
    }
}
