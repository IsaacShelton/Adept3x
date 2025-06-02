use super::Executable;
use crate::{
    Continuation, EstimateDeclScope, ExecutionCtx, Executor, FindType, GetFuncBody, GetFuncHead,
    GetTypeBody, Suspend, SuspendManyAssoc,
    execution::estimate_type_heads::EstimateTypeHeads,
    repr::{DeclScope, DeclScopeOrigin, FindTypeResult, FuncBody, FuncHead, TypeBody, TypeHead},
};
use ast_workspace::AstWorkspace;
use by_address::ByAddress;
use derivative::Derivative;
use derive_more::Debug;

#[derive(Debug, Derivative)]
#[derivative(PartialEq, Eq)]
#[debug("...")]
pub struct BuildAsg<'env> {
    pub workspace: ByAddress<&'env AstWorkspace<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(PartialEq = "ignore")]
    pub module_scopes: SuspendManyAssoc<'env, DeclScopeOrigin, &'env DeclScope>,

    #[derivative(Hash = "ignore")]
    #[derivative(PartialEq = "ignore")]
    pub estimate_type_heads: SuspendManyAssoc<'env, DeclScopeOrigin, &'env [&'env TypeHead<'env>]>,

    #[derivative(Hash = "ignore")]
    #[derivative(PartialEq = "ignore")]
    pub find_type: Suspend<'env, FindTypeResult>,

    #[derivative(Hash = "ignore")]
    #[derivative(PartialEq = "ignore")]
    pub decl_scope: Option<&'env DeclScope>,

    #[derivative(Hash = "ignore")]
    #[derivative(PartialEq = "ignore")]
    pub type_body: Suspend<'env, &'env TypeBody<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(PartialEq = "ignore")]
    pub func_head: Suspend<'env, &'env FuncHead<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(PartialEq = "ignore")]
    pub func_body: Suspend<'env, &'env FuncBody<'env>>,
}

impl<'env> BuildAsg<'env> {
    pub fn new(workspace: &'env AstWorkspace<'env>) -> Self {
        Self {
            workspace: ByAddress(workspace),
            module_scopes: None,
            estimate_type_heads: None,
            find_type: None,
            decl_scope: None,
            type_body: None,
            func_head: None,
            func_body: None,
        }
    }
}

impl<'env> Executable<'env> for BuildAsg<'env> {
    type Output = &'env asg::Asg<'env>;

    #[allow(unused_variables)]
    fn execute(
        mut self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        let workspace = self.workspace.0;

        if let Some(func_body) = executor.demand(self.func_body) {
            dbg!(func_body);
            return Ok(ctx.alloc(asg::Asg::new(self.workspace.0)));
        }

        if let Some(func_head) = executor.demand(self.func_head) {
            // dbg!(func_head);

            let func_ref = self
                .workspace
                .symbols
                .all_funcs
                .iter()
                .filter(|(_, func)| func.head.name == "exampleFunction")
                .map(|(func_ref, _)| func_ref)
                .next()
                .unwrap();

            return suspend!(
                self.func_body,
                executor.request(GetFuncBody::new(
                    workspace,
                    func_ref,
                    self.decl_scope.unwrap()
                )),
                ctx
            );
        }

        if let Some(type_body) = executor.demand(self.type_body) {
            // dbg!(type_body);

            let func_ref = self
                .workspace
                .symbols
                .all_funcs
                .iter()
                .filter(|(_, func)| func.head.name == "exampleFunction")
                .map(|(func_ref, _)| func_ref)
                .next()
                .unwrap();

            return suspend!(
                self.func_head,
                executor.request(GetFuncHead::new(
                    workspace,
                    func_ref,
                    self.decl_scope.unwrap()
                )),
                ctx
            );
        }

        if let Some(found) = executor.demand(self.find_type) {
            // dbg!(&found);

            if let Ok(Some(type_decl_ref)) = found {
                return suspend!(
                    self.type_body,
                    executor.request(GetTypeBody::new(
                        workspace,
                        self.decl_scope.unwrap(),
                        type_decl_ref
                    )),
                    ctx
                );
            } else {
                return Ok(ctx.alloc(asg::Asg::new(self.workspace.0)));
            }
        }

        if let Some(_type_heads) = executor.demand_many_assoc(&self.estimate_type_heads) {
            return suspend!(
                self.find_type,
                executor.request(FindType::new(
                    workspace,
                    self.decl_scope.unwrap(),
                    "MyTrait",
                    0
                )),
                ctx
            );
        }

        if let Some(scopes) = executor.demand_many_assoc(&self.module_scopes) {
            let first_module_ref = self.workspace.modules.keys().next().unwrap();
            self.decl_scope = scopes
                .iter()
                .find(|scope| scope.0 == DeclScopeOrigin::Module(first_module_ref))
                .map(|(_k, v)| &**v);

            return suspend_many_assoc!(
                self.estimate_type_heads,
                scopes
                    .iter()
                    .map(|(origin, scope)| (
                        *origin,
                        executor.request(EstimateTypeHeads::new(workspace, scope, "Test"))
                    ))
                    .collect(),
                ctx
            );
        }

        suspend_many_assoc!(
            self.module_scopes,
            workspace
                .modules
                .keys()
                .map(|module_ref| {
                    let scope_origin = DeclScopeOrigin::Module(module_ref);

                    (
                        scope_origin,
                        executor.request(EstimateDeclScope {
                            workspace: self.workspace,
                            scope_origin,
                        }),
                    )
                })
                .collect(),
            ctx
        )
    }
}
