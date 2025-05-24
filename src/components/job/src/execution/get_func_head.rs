use super::{Executable, ResolveType};
use crate::{
    Continuation, ExecutionCtx, Executor, SuspendMany,
    repr::{
        DeclScope, FuncHead, FuncMetadata, ImplParams, Param, Params, TargetAbi, Type, TypeKind,
    },
};
use ast_workspace::{AstWorkspace, FuncRef};
use by_address::ByAddress;
use derivative::Derivative;
use indexmap::IndexMap;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct GetFuncHead<'env> {
    func_ref: FuncRef,

    #[derivative(Debug = "ignore")]
    workspace: ByAddress<&'env AstWorkspace<'env>>,

    #[derivative(Hash = "ignore")]
    decl_scope: ByAddress<&'env DeclScope>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    inner_types: SuspendMany<'env, &'env Type<'env>>,
}

impl<'env> GetFuncHead<'env> {
    pub fn new(
        workspace: &'env AstWorkspace<'env>,
        func_ref: FuncRef,
        decl_scope: &'env DeclScope,
    ) -> Self {
        Self {
            workspace: ByAddress(workspace),
            func_ref,
            decl_scope: ByAddress(decl_scope),
            inner_types: None,
        }
    }
}

impl<'env> Executable<'env> for GetFuncHead<'env> {
    type Output = &'env FuncHead<'env>;

    fn execute(
        self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        let def = &self.workspace.symbols.all_funcs[self.func_ref];

        let Some(inner_types) = executor.demand_many(&self.inner_types) else {
            let all_inner_types = def
                .head
                .params
                .required
                .iter()
                .map(|param| &param.ast_type)
                .chain(std::iter::once(&def.head.return_type))
                .chain(def.head.givens.iter().map(|given| &given.ty))
                .map(|ast_type| ResolveType::new(&self.workspace, ast_type, &self.decl_scope));

            return suspend_many!(
                self.inner_types,
                executor.request_many(all_inner_types),
                ctx
            );
        };

        let mut inner_types = inner_types.iter();

        let params = Params {
            required: ctx.alloc_slice_fill_iter(def.head.params.required.iter().map(|param| {
                Param {
                    name: param.name.as_ref().map(|name| name.as_str()),
                    ty: inner_types.next().unwrap(),
                }
            })),
            is_cstyle_vararg: def.head.params.is_cstyle_vararg,
        };

        let return_type = *inner_types.next().unwrap();

        let impl_params = ImplParams {
            params: IndexMap::from_iter(def.head.givens.iter().enumerate().map(|(i, given)| {
                let ty = *inner_types.next().unwrap();

                let user_defined_type = match &ty.kind {
                    TypeKind::UserDefined(user_defined_type) => user_defined_type,
                    _ => panic!("we don't share error messages yet for when we expect an impl param to be a user-defined type"),
                };

                let name = given
                    .name
                    .as_ref()
                    .map(|sourced_name| sourced_name.inner().as_str())
                    .unwrap_or_else(|| ctx.alloc(format!(".{}", i)));

                (name, user_defined_type)
            })),
        };

        Ok(ctx.alloc(FuncHead {
            name: def.head.name.as_str(),
            type_params: def.head.type_params.clone(),
            params,
            return_type,
            impl_params,
            source: def.head.source,
            metadata: FuncMetadata {
                abi: def
                    .head
                    .abide_abi
                    .then_some(TargetAbi::C)
                    .unwrap_or(TargetAbi::Abstract),
                ownership: def.head.ownership,
                tag: def.head.tag,
            },
        }))
    }
}
