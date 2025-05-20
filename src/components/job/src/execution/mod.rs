mod build_asg;
mod diverge;
mod estimate_decl_scope;
mod get_type_head;
mod print;

use crate::{Artifact, Continuation, Executor, TaskRef, UnwrapFrom};
pub use build_asg::BuildAsg;
pub use diverge::Diverge;
use enum_dispatch::enum_dispatch;
pub use estimate_decl_scope::EstimateDeclScope;
pub use get_type_head::GetTypeHead;
pub use print::Print;

#[enum_dispatch]
pub trait RawExecutable<'env> {
    #[must_use]
    fn execute_raw(self, executor: &Executor<'env>) -> Result<Artifact<'env>, Continuation<'env>>;
}

pub trait Executable<'env> {
    type Output: Into<Artifact<'env>> + UnwrapFrom<Artifact<'env>>;

    #[must_use]
    fn execute(self, executor: &Executor<'env>) -> Result<Self::Output, Continuation<'env>>;
}

#[enum_dispatch]
pub trait Spawnable<'env> {
    fn spawn(&self) -> (Vec<TaskRef<'env>>, Execution<'env>);
}

#[derive(Debug)]
#[enum_dispatch(RawExecutable)]
pub enum Execution<'env> {
    Diverge(Diverge),
    Print(Print<'env>),
    BuildAsg(BuildAsg<'env>),
    EstimateDeclScope(EstimateDeclScope<'env>),
    GetTypeHead(GetTypeHead<'env>),
}

#[derive(Debug, PartialEq, Eq, Hash)]
#[enum_dispatch(Spawnable)]
pub enum Request<'env> {
    Diverge(Diverge),
    Print(Print<'env>),
    EstimateDeclScope(EstimateDeclScope<'env>),
    GetTypeHead(GetTypeHead<'env>),
}

impl<'env, E> RawExecutable<'env> for E
where
    E: Executable<'env>,
{
    fn execute_raw(self, executor: &Executor<'env>) -> Result<Artifact<'env>, Continuation<'env>> {
        self.execute(executor).map(|value| value.into())
    }
}
