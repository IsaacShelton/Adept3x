use super::{Executable, Execution, Spawnable};
use crate::{
    Continuation, EstimateDeclScope, Executor, Pending, TaskRef,
    repr::{DeclScope, DeclScopeOrigin},
};
use ast_workspace::AstWorkspace;
use by_address::ByAddress;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildAsg<'env> {
    pub workspace: ByAddress<&'env AstWorkspace<'env>>,
    pub scopes: Option<Vec<Pending<'env, DeclScope>>>,
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
            let mut scopes = vec![];

            for module_ref in workspace.modules.keys() {
                let new_scope = executor.request(EstimateDeclScope {
                    workspace: self.workspace,
                    scope_origin: DeclScopeOrigin::Module(module_ref),
                });
                scopes.push(new_scope);
            }

            return Err(Continuation::suspend(
                scopes.iter().map(|scope| scope.raw_task_ref()).collect(),
                Self {
                    scopes: Some(scopes),
                    ..self
                },
            ));
        };

        let truth = executor.truth.read().unwrap();
        let scopes = scopes
            .into_iter()
            .copied()
            .map(|scope| truth.demand(scope))
            .collect::<Vec<_>>();

        dbg!(scopes);

        Ok(asg::Asg::new(self.workspace.0))
    }
}

impl<'env> Spawnable<'env> for BuildAsg<'env> {
    fn spawn(&self) -> (Vec<TaskRef<'env>>, Execution<'env>) {
        (vec![], self.clone().into())
    }
}
