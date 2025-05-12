use super::Execute;
use crate::{Artifact, Executor, Progress};
use ast_workspace::AstWorkspace;

#[derive(Debug)]
pub struct BuildAstWorkspace<'env>(pub &'env AstWorkspace<'env>);

impl<'env> Execute<'env> for BuildAstWorkspace<'env> {
    fn execute(self, _executor: &Executor<'env>) -> Progress<'env> {
        Artifact::AstWorkspace(self.0).into()
    }
}
