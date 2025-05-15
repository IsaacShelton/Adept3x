use super::Execute;
use crate::{Artifact, Executor, Progress, TaskRef, prereqs::Prereqs};
use ast_workspace::{AstWorkspace, StructRef};
use by_address::ByAddress;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BuildAsgForStruct<'env> {
    workspace: ByAddress<&'env AstWorkspace<'env>>,
    ast_struct_ref: StructRef,
}

impl<'env> BuildAsgForStruct<'env> {
    pub fn new(workspace: ByAddress<&'env AstWorkspace<'env>>, ast_struct_ref: StructRef) -> Self {
        Self {
            workspace,
            ast_struct_ref,
        }
    }
}

impl<'env> Prereqs<'env> for BuildAsgForStruct<'env> {
    fn prereqs(&self) -> Vec<TaskRef<'env>> {
        vec![]
    }
}

impl<'env> Execute<'env> for BuildAsgForStruct<'env> {
    fn execute(self, _executor: &Executor<'env>) -> Progress<'env> {
        let workspace = self.workspace;
        let structure = &workspace.all_structs[self.ast_struct_ref];
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
