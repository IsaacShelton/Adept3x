use super::{Executable, GetTypeHead};
use crate::{
    BumpAllocator, Continuation, Executor, SuspendMany,
    repr::{DeclScope, TypeHead},
};
use ast_workspace::AstWorkspace;
use by_address::ByAddress;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EstimateTypeHeads<'env> {
    workspace: ByAddress<&'env AstWorkspace<'env>>,
    decl_scope: ByAddress<&'env DeclScope>,
    name: &'env str,
    type_head_tasks: SuspendMany<'env, &'env TypeHead<'env>>,
}

impl<'env> EstimateTypeHeads<'env> {
    pub fn new(
        workspace: &'env AstWorkspace<'env>,
        decl_scope: &'env DeclScope,
        name: &'env str,
    ) -> Self {
        Self {
            workspace: ByAddress(workspace),
            decl_scope: ByAddress(decl_scope),
            name,
            type_head_tasks: None,
        }
    }
}

impl<'env> Executable<'env> for EstimateTypeHeads<'env> {
    type Output = &'env [&'env TypeHead<'env>];

    fn execute(
        self,
        executor: &Executor<'env>,
        allocator: &'env BumpAllocator,
    ) -> Result<Self::Output, Continuation<'env>> {
        let workspace = self.workspace.0;

        if let Some(type_head_tasks) = self.type_head_tasks {
            return Ok(allocator.alloc_slice_fill_iter(
                executor
                    .truth
                    .read()
                    .unwrap()
                    .demand_many(type_head_tasks.iter())
                    .into_iter(),
            ));
        }

        let decl_set = self.decl_scope.get(&self.name);

        suspend_many!(
            self.type_head_tasks,
            decl_set
                .into_iter()
                .flat_map(|decl_set| decl_set.type_decls())
                .map(|type_decl_ref| executor.request(GetTypeHead::new(workspace, type_decl_ref)))
                .collect()
        )
    }
}
