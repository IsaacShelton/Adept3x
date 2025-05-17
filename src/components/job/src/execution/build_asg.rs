use super::{
    Execute,
    estimate_decl_scope::{DeclScopeOrigin, EstimateDeclScope},
};
use crate::{Artifact, BuildAsgForStruct, Executor, Progress, TaskRef};
use asg::Asg;
use ast_workspace::AstWorkspace;
use by_address::ByAddress;

#[derive(Debug)]
pub struct BuildAsg<'env> {
    pub workspace: ByAddress<&'env AstWorkspace<'env>>,
    pub fanned_out: bool,
    pub structs: Vec<TaskRef<'env>>,
    pub scopes: Vec<TaskRef<'env>>,
}

impl<'env> BuildAsg<'env> {
    pub fn new(workspace: &'env AstWorkspace<'env>) -> Self {
        Self {
            workspace: ByAddress(workspace),
            fanned_out: false,
            structs: Vec::new(),
            scopes: Vec::new(),
        }
    }
}

impl<'env> Execute<'env> for BuildAsg<'env> {
    fn execute(self, executor: &Executor<'env>) -> Progress<'env> {
        let workspace = self.workspace;

        if !self.fanned_out {
            let mut suspend_on = vec![];
            let mut structs = vec![];
            let mut scopes = vec![];

            for module_ref in workspace.modules.keys() {
                let new_scope = executor.request(EstimateDeclScope {
                    workspace: self.workspace,
                    scope_origin: DeclScopeOrigin::Module(module_ref),
                });
                scopes.push(new_scope);
                suspend_on.push(new_scope);
            }

            for (ast_struct_ref, _) in &workspace.symbols.all_structs {
                let spawned = executor.request(BuildAsgForStruct::new(workspace, ast_struct_ref));
                structs.push(spawned);
                suspend_on.push(spawned);
            }

            return Progress::suspend(
                suspend_on,
                Self {
                    workspace: self.workspace,
                    fanned_out: true,
                    structs,
                    scopes,
                },
            );
        }

        {
            let truth = executor.truth.read().unwrap();
            for scope in self.scopes.iter().copied() {
                let estimated_decl_scope =
                    truth.expect_artifact(scope).unwrap_estimated_decl_scope();
                dbg!(&estimated_decl_scope);
            }
        }

        let asg = Asg::new(*workspace);
        Artifact::Asg(asg).into()
    }
}
