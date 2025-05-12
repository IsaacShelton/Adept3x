use super::Execute;
use crate::{Artifact, Executor, Progress, TaskRef};
use ast_workspace::StructRef;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BuildAsgForStruct<'outside> {
    ast_workspace: TaskRef<'outside>,
    ast_struct_ref: StructRef,
}

impl<'outside> BuildAsgForStruct<'outside> {
    pub fn new(ast_workspace: TaskRef<'outside>, ast_struct_ref: StructRef) -> Self {
        Self {
            ast_workspace,
            ast_struct_ref,
        }
    }

    pub fn suspend_on(&self) -> Vec<TaskRef<'outside>> {
        vec![self.ast_workspace]
    }
}

impl<'outside> Execute<'outside> for BuildAsgForStruct<'outside> {
    fn execute(self, executor: &Executor<'outside>) -> Progress<'outside> {
        let ast_workspace = executor.truth.read().unwrap().tasks[self.ast_workspace]
            .state
            .completed()
            .unwrap()
            .unwrap_ast_workspace();

        let structure = &ast_workspace.all_structs[self.ast_struct_ref];
        println!("PROCESSING AST STRUCT: '{}'", structure.name);

        // This is going to suspend on idenfier lookup
        // And we expect each to give us back a TypeRef essentially

        // We could for example, have each worker have it's own arena, and have a
        // combined TypeRef that also says which arena to find it in, but maybe that wouldn't work
        // actually,

        // We will somehow have to keep a registery of all the created types,
        // which "identifier modules" can then reference and we can lookup from for here.

        Artifact::Void.into()
    }
}
