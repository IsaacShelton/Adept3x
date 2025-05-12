use super::Execute;
use crate::{Artifact, Executor, Progress, TaskRef, prereqs::Prereqs};
use ast_workspace::StructRef;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BuildAsgForStruct<'env> {
    ast_workspace: TaskRef<'env>,
    ast_struct_ref: StructRef,
}

impl<'env> BuildAsgForStruct<'env> {
    pub fn new(ast_workspace: TaskRef<'env>, ast_struct_ref: StructRef) -> Self {
        Self {
            ast_workspace,
            ast_struct_ref,
        }
    }
}

impl<'env> Prereqs<'env> for BuildAsgForStruct<'env> {
    fn prereqs(&self) -> Vec<TaskRef<'env>> {
        vec![self.ast_workspace]
    }
}

impl<'env> Execute<'env> for BuildAsgForStruct<'env> {
    fn execute(self, executor: &Executor<'env>) -> Progress<'env> {
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
