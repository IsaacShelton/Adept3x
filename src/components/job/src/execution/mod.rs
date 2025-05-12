mod build_asg;
mod build_asg_for_struct;
mod build_ast_workspace;
mod build_static_scope;
mod create_string;
mod identifiers_hashmap;
mod infin;
mod print;
mod print_message;

use crate::{Executor, Progress};
pub use build_asg::*;
pub use build_asg_for_struct::*;
pub use build_ast_workspace::BuildAstWorkspace;
pub use build_static_scope::BuildStaticScope;
pub use create_string::*;
use enum_dispatch::enum_dispatch;
pub use identifiers_hashmap::IdentifiersHashMap;
pub use infin::Infin;
pub use print::*;
pub use print_message::*;

#[enum_dispatch]
pub trait Execute<'outside> {
    #[must_use]
    fn execute(self, executor: &Executor<'outside>) -> Progress<'outside>;
}

#[derive(Debug)]
#[enum_dispatch(Execute)]
pub enum Execution<'outside> {
    CreateString(CreateString),
    Print(Print<'outside>),
    PrintMessage(PrintMessage<'outside>),
    Infin(Infin),
    IdentifiersHashMap(IdentifiersHashMap<'outside>),
    BuildAstWorkspace(BuildAstWorkspace<'outside>),
    BuildAsg(BuildAsg<'outside>),
    BuildAsgForStruct(BuildAsgForStruct<'outside>),
    BuildStaticScope(BuildStaticScope<'outside>), // ----------
}
