use super::{Executable, GetTypeHead};
use crate::{
    Continuation, EstimateDeclScope, ExecutionCtx, Executor, FindType, Suspend, SuspendMany,
    execution::estimate_type_heads::EstimateTypeHeads,
    repr::{DeclScope, DeclScopeOrigin, FindTypeResult, TypeHead},
};
use ast_workspace::AstWorkspace;
use by_address::ByAddress;
use derive_more::Debug;

#[derive(Debug, Clone, PartialEq, Eq)]
#[debug("...")]
pub struct BuildAsg<'env> {
    pub workspace: ByAddress<&'env AstWorkspace<'env>>,
    pub scopes: SuspendMany<'env, &'env DeclScope>,
    pub test_estimate_type_heads: SuspendMany<'env, &'env [&'env TypeHead<'env>]>,
    pub find_type: Suspend<'env, FindTypeResult>,
}

impl<'env> BuildAsg<'env> {
    pub fn new(workspace: &'env AstWorkspace<'env>) -> Self {
        Self {
            workspace: ByAddress(workspace),
            scopes: None,
            test_estimate_type_heads: None,
            find_type: None,
        }
    }
}

impl<'env> Executable<'env> for BuildAsg<'env> {
    type Output = &'env asg::Asg<'env>;

    fn execute(
        self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        let workspace = self.workspace.0;

        if let Some(found) = executor.demand(self.find_type) {
            dbg!(&found);
            return Ok(ctx.alloc(asg::Asg::new(self.workspace.0)));
        }

        if let Some(type_heads) = executor.demand_many(&self.test_estimate_type_heads) {
            dbg!(type_heads);
            let first_module_ref = workspace.modules.iter().next().unwrap().0;

            return suspend!(
                self.find_type,
                executor.request(FindType::new(
                    workspace,
                    DeclScopeOrigin::Module(first_module_ref),
                    "Test",
                    0
                )),
                ctx
            );
        }

        if let Some(scopes) = executor.demand_many(&self.scopes) {
            for module in workspace.modules.values() {
                for name_scope in module
                    .name_scopes()
                    .map(|scope| &workspace.symbols.all_name_scopes[scope])
                {
                    for type_decl_ref in name_scope.direct_type_decls() {
                        // Spawn request ahead of time
                        let _ = executor.request(GetTypeHead::new(workspace, type_decl_ref));
                    }
                }
            }

            return suspend_many!(
                self.test_estimate_type_heads,
                scopes
                    .iter()
                    .map(|scope| executor.request(EstimateTypeHeads::new(workspace, scope, "Test")))
                    .collect(),
                ctx
            );
        }

        suspend_many!(
            self.scopes,
            workspace
                .modules
                .keys()
                .map(|module_ref| {
                    executor.request(EstimateDeclScope {
                        workspace: self.workspace,
                        scope_origin: DeclScopeOrigin::Module(module_ref),
                    })
                })
                .collect(),
            ctx
        )
    }
}
