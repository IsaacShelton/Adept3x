mod build_asg;
mod build_asg_for_struct;
mod build_ast_workspace;
mod build_static_scope;
mod create_string;
mod diverge;
mod identifiers_hashmap;
mod print;
mod print_message;

use crate::{Executor, Progress, spawn_execution::SpawnExecution};
pub use build_asg::*;
pub use build_asg_for_struct::*;
pub use build_ast_workspace::BuildAstWorkspace;
pub use build_static_scope::BuildStaticScope;
pub use create_string::*;
pub use diverge::Diverge;
use enum_dispatch::enum_dispatch;
pub use identifiers_hashmap::IdentifiersHashMap;
pub use print::*;
pub use print_message::*;

#[enum_dispatch]
pub trait Execute<'env> {
    #[must_use]
    fn execute(self, executor: &Executor<'env>) -> Progress<'env>;
}

#[derive(Debug)]
#[enum_dispatch(Execute)]
pub enum Execution<'env> {
    CreateString(CreateString),
    Print(Print<'env>),
    PrintMessage(PrintMessage<'env>),
    Diverge(Diverge),
    IdentifiersHashMap(IdentifiersHashMap<'env>),
    BuildAstWorkspace(BuildAstWorkspace<'env>),
    BuildAsg(BuildAsg<'env>),
    BuildAsgForStruct(BuildAsgForStruct<'env>),
    BuildStaticScope(BuildStaticScope<'env>),
}

impl<'env, E> SpawnExecution<'env> for E
where
    E: Clone + Into<Execution<'env>>,
{
    fn spawn_execution(&self) -> Execution<'env> {
        self.clone().into()
    }
}
