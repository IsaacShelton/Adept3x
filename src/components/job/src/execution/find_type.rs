use super::{Executable, FindTypeInEstimated};
use crate::{BumpAllocator, Continuation, Executor, Suspend, repr::DeclScope};
use ast_workspace::AstWorkspace;
use by_address::ByAddress;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FindType<'env> {
    workspace: ByAddress<&'env AstWorkspace<'env>>,
    decl_scope: ByAddress<&'env DeclScope>,
    name: &'env str,
    arity: usize,
    estimation_pass: Suspend<'env, Option<&'env asg::Type>>,
}

impl<'env> FindType<'env> {
    pub fn new(
        workspace: &'env AstWorkspace<'env>,
        decl_scope: &'env DeclScope,
        name: &'env str,
        arity: usize,
    ) -> Self {
        Self {
            workspace: ByAddress(workspace),
            decl_scope: ByAddress(decl_scope),
            name,
            arity,
            estimation_pass: None,
        }
    }
}

impl<'env> Executable<'env> for FindType<'env> {
    type Output = Option<&'env asg::Type>;

    fn execute(
        self,
        executor: &Executor<'env>,
        _allocator: &'env BumpAllocator,
    ) -> Result<Self::Output, Continuation<'env>> {
        let workspace = self.workspace.0;

        if let Some(estimation_pass) = self.estimation_pass {
            let found = *executor.truth.read().unwrap().demand(estimation_pass);
            return Ok(found);
        }

        suspend!(
            self.estimation_pass,
            executor.request(FindTypeInEstimated::new(
                workspace,
                &self.decl_scope,
                self.name,
                self.arity,
            ))
        )
    }
}
