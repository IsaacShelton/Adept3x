use super::Execute;
use crate::{Artifact, Executor, Progress};
use ast_workspace::AstWorkspace;

#[derive(Debug)]
pub struct BuildAstWorkspace<'outside>(pub &'outside AstWorkspace<'outside>);

impl<'outside> Execute<'outside> for BuildAstWorkspace<'outside> {
    fn execute(self, _executor: &Executor<'outside>) -> Progress<'outside> {
        Artifact::AstWorkspace(self.0).into()
    }
}
