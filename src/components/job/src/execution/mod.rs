mod find_type;
mod io;
mod lower;
mod main;
mod process;
mod resolve;
mod util;

use crate::{
    Artifact, Continuation, ExecutionCtx, Executor, TaskRef, UnwrapFrom,
    execution::{
        lower::{LowerFunctionBody, LowerFunctionHead, LowerType},
        resolve::{
            EvaluateComptime, ResolveEvaluation, ResolveFunctionBody, ResolveFunctionHead,
            ResolveType,
        },
    },
};
use enum_dispatch::enum_dispatch;
pub use find_type::FindType;
pub use io::*;
pub use main::Main;
pub use process::*;
pub use util::*;

#[enum_dispatch]
pub trait RawExecutable<'env> {
    #[must_use]
    fn execute_raw(
        self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Artifact<'env>, Continuation<'env>>;
}

pub trait Executable<'env> {
    type Output: Into<Artifact<'env>> + UnwrapFrom<Artifact<'env>>;

    #[must_use]
    fn execute(
        self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>>;
}

#[enum_dispatch]
pub trait Spawnable<'env> {
    fn spawn(&self) -> (Vec<TaskRef<'env>>, Execution<'env>);
}

impl<'env, T> Spawnable<'env> for T
where
    T: Clone + Into<Execution<'env>>,
{
    fn spawn(&self) -> (Vec<TaskRef<'env>>, Execution<'env>) {
        (vec![], self.clone().into())
    }
}

#[derive(Debug)]
#[enum_dispatch(RawExecutable)]
pub enum Execution<'env> {
    Main(Main<'env>),

    // Utility
    Diverge(Diverge),
    Print(Print<'env>),

    // Prototypes
    //FindType(FindType<'env>),
    //GetTypeHead(GetTypeHead<'env>),
    //GetTypeBody(GetTypeBody<'env>),
    //ResolveTypeKeepAliases(ResolveTypeKeepAliases<'env>),
    //ResolveTypeArg(ResolveTypeArg<'env>),
    //GetFuncHead(GetFuncHead<'env>),
    //GetFuncBody(GetFuncBody<'env>),

    // Processing
    ProcessFile(ProcessFile<'env>),
    ProcessNamespaceItems(ProcessNamespaceItems<'env>),
    ProcessNamespace(ProcessNamespace<'env>),
    ProcessWhen(ProcessWhen<'env>),
    ProcessPragma(ProcessPragma<'env>),
    ProcessLinkset(ProcessLinkset<'env>),
    ProcessStructure(ProcessStructure<'env>),

    // Resolution
    ResolveType(ResolveType<'env>),
    ResolveEvaluation(ResolveEvaluation<'env>),
    ResolveFunction(ProcessFunction<'env>),
    ResolveFunctionHead(ResolveFunctionHead<'env>),
    ResolveFunctionBody(ResolveFunctionBody<'env>),

    // Comptime
    EvaluateComptime(EvaluateComptime<'env>),

    // Lowering
    LowerType(LowerType<'env>),
    LowerFunctionHead(LowerFunctionHead<'env>),
    LowerFunctionBody(LowerFunctionBody<'env>),
}

#[derive(Debug, PartialEq, Eq, Hash)]
#[enum_dispatch(Spawnable)]
pub enum Request<'env> {
    // Utility
    Diverge(Diverge),
    Print(Print<'env>),

    // Prototypes
    //FindType(FindType<'env>),
    //GetTypeHead(GetTypeHead<'env>),
    //GetTypeBody(GetTypeBody<'env>),
    //ResolveTypeKeepAliases(ResolveTypeKeepAliases<'env>),
    //ResolveTypeArg(ResolveTypeArg<'env>),
    //GetFuncHead(GetFuncHead<'env>),
    //GetFuncBody(GetFuncBody<'env>),

    // Processing
    ProcessFile(ProcessFile<'env>),
    ProcessNamespaceItems(ProcessNamespaceItems<'env>),
    ProcessNamespace(ProcessNamespace<'env>),
    ProcessWhen(ProcessWhen<'env>),
    ProcessPragma(ProcessPragma<'env>),
    ProcessLinkset(ProcessLinkset<'env>),
    ProcessStructure(ProcessStructure<'env>),

    // Resolution
    ResolveType(ResolveType<'env>),
    ResolveEvaluation(ResolveEvaluation<'env>),
    ResolveFunction(ProcessFunction<'env>),
    ResolveFunctionHead(ResolveFunctionHead<'env>),
    ResolveFunctionBody(ResolveFunctionBody<'env>),

    // Comptime
    EvaluateComptime(EvaluateComptime<'env>),

    // Lowering
    LowerType(LowerType<'env>),
    LowerFunctionHead(LowerFunctionHead<'env>),
    LowerFunctionBody(LowerFunctionBody<'env>),
}

impl<'env, E> RawExecutable<'env> for E
where
    E: Executable<'env>,
{
    fn execute_raw(
        self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Artifact<'env>, Continuation<'env>> {
        self.execute(executor, ctx).map(|value| value.into())
    }
}
