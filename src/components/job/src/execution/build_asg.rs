use super::Executable;
use crate::{
    Continuation, EstimateDeclScope, ExecutionCtx, Executor, FindType, GetTypeBody, Suspend,
    SuspendManyAssoc,
    execution::estimate_type_heads::EstimateTypeHeads,
    repr::{DeclScope, DeclScopeOrigin, FindTypeResult, TypeBody, TypeHead},
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
        }
    }
}

impl<'env> Executable<'env> for BuildAsg<'env> {
    type Output = &'env asg::Asg<'env>;

    fn execute(
        mut self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        let workspace = self.workspace.0;

        if let Some(type_body) = executor.demand(self.type_body) {
            dbg!("{:?}", type_body);
            return Ok(ctx.alloc(asg::Asg::new(self.workspace.0)));
        }

        if let Some(found) = executor.demand(self.find_type) {
            dbg!(&found);

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
                    "Test",
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
