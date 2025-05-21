use super::{Executable, GetTypeHead, estimate_type_heads::EstimateTypeHeads};
use crate::{
    BumpAllocator, Continuation, EstimateDeclScope, Executor, SuspendMany,
    repr::{DeclScope, DeclScopeOrigin, TypeHead},
};
use ast_workspace::AstWorkspace;
use by_address::ByAddress;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildAsg<'env> {
    pub workspace: ByAddress<&'env AstWorkspace<'env>>,
    pub scopes: SuspendMany<'env, &'env DeclScope>,
    pub test_estimate_type_heads: SuspendMany<'env, &'env [&'env TypeHead<'env>]>,
}

impl<'env> BuildAsg<'env> {
    pub fn new(workspace: &'env AstWorkspace<'env>) -> Self {
        Self {
            workspace: ByAddress(workspace),
            scopes: None,
            test_estimate_type_heads: None,
        }
    }
}

impl<'env> Executable<'env> for BuildAsg<'env> {
    type Output = &'env asg::Asg<'env>;

    fn execute(
        self,
        executor: &Executor<'env>,
        allocator: &'env BumpAllocator,
    ) -> Result<Self::Output, Continuation<'env>> {
        let workspace = self.workspace.0;

        if let Some(test_estimates) = self.test_estimate_type_heads {
            let truth = executor.truth.read().unwrap();
            let type_heads = truth.demand_many(test_estimates.iter());
            dbg!(type_heads);
            return Ok(allocator.alloc(asg::Asg::new(self.workspace.0)));
        }

        if let Some(scopes) = self.scopes.as_ref() {
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

            let scopes = executor.truth.read().unwrap().demand_many(scopes.iter());

            return suspend_many!(
                self.test_estimate_type_heads,
                scopes
                    .iter()
                    .map(|scope| executor.request(EstimateTypeHeads::new(workspace, scope, "Test")))
                    .collect()
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
                .collect()
        )
    }
}
