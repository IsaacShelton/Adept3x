use super::Executable;
use crate::{
    Continuation, ExecutionCtx, Executor, SuspendMany,
    module_graph::ModuleView,
    repr::{
        FuncHead, FuncMetadata, ImplParams, Param, Params, TargetAbi, Type, TypeKind,
        UnaliasedType, UnaliasedUserDefinedType,
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

    #[derivative(Debug = "ignore")]
    view: ModuleView<'env>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    inner_types: SuspendMany<'env, &'env Type<'env>>,
}

impl<'env> GetFuncHead<'env> {
    pub fn new(
        workspace: &'env AstWorkspace<'env>,
        view: ModuleView<'env>,
        func_ref: FuncRef,
    ) -> Self {
        Self {
            workspace: ByAddress(workspace),
            func_ref,
            view,
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
                .map(|ast_type| ResolveTypeKeepAliases::new(&self.workspace, ast_type, self.view));

            return suspend_many!(
                self.inner_types,
                executor.request_many(all_inner_types),
                ctx
            );
        };

        let mut inner_types = inner_types.iter();

        let unalias = |ty: &'env Type<'env>| -> UnaliasedType<'env> {
            if ty.contains_type_alias() {
                // This will need to be able to suspend
                todo!("unalias type in GetFuncHead")
            } else {
                UnaliasedType(ty)
            }
        };

        let params = Params {
            required: ctx.alloc_slice_fill_iter(def.head.params.required.iter().map(|param| {
                Param {
                    name: param.name.as_ref().map(|name| name.as_str()),
                    ty: unalias(inner_types.next().unwrap()),
                }
            })),
            is_cstyle_vararg: def.head.params.is_cstyle_vararg,
        };

        let return_type = unalias(inner_types.next().unwrap());

        let impl_params = ImplParams {
            params: IndexMap::from_iter(def.head.givens.iter().enumerate().map(|(i, given)| {
                let ty = unalias(inner_types.next().unwrap());

                let user_defined_type = match &ty.0.kind {
                    TypeKind::UserDefined(user_defined_type) => user_defined_type,
                    _ => panic!("we don't share error messages yet for when we expect an impl param to be a user-defined type"),
                };

                let name = given
                    .name
                    .as_ref()
                    .map(|sourced_name| sourced_name.inner().as_str())
                    .unwrap_or_else(|| ctx.alloc(format!(".{}", i)));

                (name, UnaliasedUserDefinedType(user_defined_type))
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
