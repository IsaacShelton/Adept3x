use super::{Execute, build_static_scope::BuildStaticScope};
use crate::{Artifact, BuildAsgForStruct, Executor, Progress, TaskRef};
use asg::Asg;

#[derive(Debug)]
pub struct BuildAsg<'env> {
    pub ast_workspace_task_ref: TaskRef<'env>,
    pub fanned_out: bool,
    pub structs: Vec<TaskRef<'env>>,
    pub scopes: Vec<TaskRef<'env>>,
}

impl<'env> BuildAsg<'env> {
    pub fn new(ast_workspace_task_ref: TaskRef<'env>) -> Self {
        Self {
            ast_workspace_task_ref,
            fanned_out: false,
            structs: Vec::new(),
            scopes: Vec::new(),
        }
    }
}

impl<'env> Execute<'env> for BuildAsg<'env> {
    fn execute(self, executor: &Executor<'env>) -> Progress<'env> {
        let ast_workspace = {
            let truth = executor.truth.read().unwrap();

            let Some(Artifact::AstWorkspace(ast_workspace)) =
                truth.tasks[self.ast_workspace_task_ref].state.completed()
            else {
                panic!("BuildAsg task expected completed AstWorkspace before running!");
            };

            *ast_workspace
        };

        if !self.fanned_out {
            let mut suspend_on = vec![];
            let mut structs = vec![];
            let mut scopes = vec![];

            for (fs_node_id, ast_file) in &ast_workspace.files {
                if ast_file.is_none() {
                    continue;
                }

                let new_scope = executor.request(BuildStaticScope {
                    ast_workspace: self.ast_workspace_task_ref,
                    fs_node_id: fs_node_id.into_raw(),
                });
                scopes.push(new_scope);
                suspend_on.push(new_scope);
            }

            for (ast_struct_ref, _) in &ast_workspace.all_structs {
                let spawned = executor.request(BuildAsgForStruct::new(
                    self.ast_workspace_task_ref,
                    ast_struct_ref,
                ));
                structs.push(spawned);
                suspend_on.push(spawned);
            }

            return Progress::suspend(
                suspend_on,
                Self {
                    ast_workspace_task_ref: self.ast_workspace_task_ref,
                    fanned_out: true,
                    structs,
                    scopes,
                },
            );
        }

        {
            let truth = executor.truth.read().unwrap();
            for scope in self.scopes.iter().copied() {
                let static_scope = truth.tasks[scope]
                    .state
                    .unwrap_completed()
                    .unwrap_static_scope();
                dbg!(&static_scope);
            }
        }

        let asg = Asg::new(ast_workspace);
        Artifact::Asg(asg).into()
    }
}
