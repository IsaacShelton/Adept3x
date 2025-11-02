use crate::{
    Continuation, Executable, ExecutionCtx, Executor, SuspendMany,
    execution::resolve::ResolveType,
    module_graph::ModuleView,
    repr::{DeclHead, FuncHead, FuncMetadata, ImplParams, Param, Params, TargetAbi, UnaliasedType},
};
use attributes::{Exposure, SymbolOwnership, Tag};
use by_address::ByAddress;
use derivative::Derivative;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct ResolveFunctionHead<'env> {
    view: &'env ModuleView<'env>,
    ast_func: ByAddress<&'env ast::Func>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    inner_types: SuspendMany<'env, UnaliasedType<'env>>,
}

impl<'env> ResolveFunctionHead<'env> {
    pub fn new(view: &'env ModuleView<'env>, ast_func: &'env ast::Func) -> Self {
        Self {
            view,
            ast_func: ByAddress(ast_func),
            inner_types: None,
        }
    }
}

impl<'env> Executable<'env> for ResolveFunctionHead<'env> {
    type Output = &'env FuncHead<'env>;

    fn execute(
        self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        let ast_head = &self.ast_func.head;

        let Some(inner_types) = executor.demand_many(&self.inner_types) else {
            let suspend_on_types = ast_head
                .params
                .required
                .iter()
                .map(|param| &param.ast_type)
                .chain(std::iter::once(&ast_head.return_type));

            return suspend_many!(
                self.inner_types,
                suspend_on_types
                    .map(|ty| executor.request(ResolveType::new(self.view, ty)))
                    .collect(),
                ctx
            );
        };

        let mut inner_types = inner_types.into_iter();

        let params = Params {
            required: ctx.alloc_slice_fill_iter(ast_head.params.required.iter().map(|param| {
                Param {
                    name: param.name.as_ref().map(|name| name.as_str()),
                    ty: inner_types.next().unwrap(),
                }
            })),
            is_cstyle_vararg: ast_head.params.is_cstyle_vararg,
        };

        let return_type = inner_types.next().unwrap();

        let impl_params = ImplParams::default();
        assert_eq!(ast_head.givens.len(), 0); // We don't support impl params yet

        /*
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
        */

        let ownership = (ast_head.tag == Some(Tag::Main))
            .then_some(SymbolOwnership::Owned(Exposure::Exposed))
            .unwrap_or(ast_head.ownership);

        let func_head = ctx.alloc(FuncHead {
            name: ast_head.name.as_str(),
            type_params: ast_head.type_params.clone(),
            params,
            return_type,
            impl_params,
            source: ast_head.source,
            metadata: FuncMetadata {
                abi: ast_head
                    .abide_abi
                    .then_some(TargetAbi::C)
                    .unwrap_or(TargetAbi::Abstract),
                ownership,
                tag: ast_head.tag,
            },
            view: self.view,
            ast_func: self.ast_func.0,
        });

        self.view.add_symbol(
            ast_head.privacy,
            ast_head.name.as_str(),
            DeclHead::FuncLike(func_head),
        );

        executor.wake_pending_search(self.view.graph, &ast_head.name);
        Ok(func_head)
    }
}
