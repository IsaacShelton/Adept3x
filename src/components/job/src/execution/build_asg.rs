use super::{Executable, Execution, GetTypeHead, Spawnable};
use crate::{
    Continuation, EstimateDeclScope, Executor, Pending, TaskRef,
    repr::{DeclScope, DeclScopeOrigin},
};
use ast_workspace::{AstWorkspace, ModuleRef};
use by_address::ByAddress;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildAsg<'env> {
    pub workspace: ByAddress<&'env AstWorkspace<'env>>,
    pub scopes: Option<HashMap<ModuleRef, Pending<'env, DeclScope>>>,
}

impl<'env> BuildAsg<'env> {
    pub fn new(workspace: &'env AstWorkspace<'env>) -> Self {
        Self {
            workspace: ByAddress(workspace),
            scopes: None,
        }
    }
}

impl<'env> Executable<'env> for BuildAsg<'env> {
    type Output = asg::Asg<'env>;

    fn execute(self, executor: &Executor<'env>) -> Result<Self::Output, Continuation<'env>> {
        let workspace = self.workspace.0;

        let Some(scopes) = self.scopes.as_ref() else {
            let mut scopes = HashMap::new();

            for module_ref in workspace.modules.keys() {
                let new_scope = executor.request(EstimateDeclScope {
                    workspace: self.workspace,
                    scope_origin: DeclScopeOrigin::Module(module_ref),
                });
                scopes.insert(module_ref, new_scope);
            }

            return Err(Continuation::suspend(
                scopes.values().map(|scope| scope.raw_task_ref()).collect(),
                Self {
                    scopes: Some(scopes),
                    ..self
                },
            ));
        };

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

        {
            let truth = executor.truth.read().unwrap();
            let scopes = truth.demand_map(scopes);
            dbg!(scopes);
        }

        Ok(asg::Asg::new(self.workspace.0))
    }
}

impl<'env> Spawnable<'env> for BuildAsg<'env> {
    fn spawn(&self) -> (Vec<TaskRef<'env>>, Execution<'env>) {
        (vec![], self.clone().into())
    }
}
