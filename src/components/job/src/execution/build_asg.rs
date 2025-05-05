use super::Execute;
use crate::{Artifact, BuildAsgForStruct, Executor, Progress, TaskRef};
use asg::Asg;

#[derive(Debug)]
pub struct BuildAsg<'outside> {
    pub ast_workspace_task_ref: TaskRef<'outside>,
    pub fanned_out: bool,
    pub structs: Vec<TaskRef<'outside>>,
}

impl<'outside> BuildAsg<'outside> {
    pub fn new(ast_workspace_task_ref: TaskRef<'outside>) -> Self {
        Self {
            ast_workspace_task_ref,
            fanned_out: false,
            structs: Vec::new(),
        }
    }
}

impl<'outside> Execute<'outside> for BuildAsg<'outside> {
    fn execute(self, executor: &Executor<'outside>) -> Progress<'outside> {
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

            for (ast_struct_ref, _) in &ast_workspace.all_structs {
                let spawned = executor.request(BuildAsgForStruct::new(
                    self.ast_workspace_task_ref,
                    ast_struct_ref,
                ));
                structs.push(spawned);
                suspend_on.push(spawned);
            }

            println!("build_asg waiting on {:?}", &suspend_on);
            return Progress::suspend(
                suspend_on,
                Self {
                    ast_workspace_task_ref: self.ast_workspace_task_ref,
                    fanned_out: true,
                    structs,
                },
            );
        }

        let asg = Asg::new(ast_workspace);
        Artifact::Asg(asg).into()
    }
}
