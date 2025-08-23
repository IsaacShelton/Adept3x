mod build_ir;
mod canonicalize;
mod diverge;
mod find_type;
mod main;
mod print;
mod resolve;

use crate::{
    Artifact, Continuation, ExecutionCtx, Executor, TaskRef, UnwrapFrom,
    execution::{
        build_ir::{BuildIr, LowerFunctionHead},
        main::LoadFile,
        resolve::{
            EvaluateComptime, ResolveEvaluation, ResolveFunction, ResolveFunctionBody,
            ResolveFunctionHead, ResolveNamespace, ResolveNamespaceItems, ResolveType, ResolveWhen,
        },
    },
};
pub use canonicalize::canonicalize_or_error;
pub use diverge::Diverge;
use enum_dispatch::enum_dispatch;
pub use find_type::FindType;
pub use main::Main;
pub use print::Print;

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
    Diverge(Diverge),
    Print(Print<'env>),
    FindType(FindType<'env>),
    // GetTypeHead(GetTypeHead<'env>),
    // GetTypeBody(GetTypeBody<'env>),
    ResolveType(ResolveType<'env>),
    // ResolveTypeKeepAliases(ResolveTypeKeepAliases<'env>),
    // ResolveTypeArg(ResolveTypeArg<'env>),
    //GetFuncHead(GetFuncHead<'env>),
    //GetFuncBody(GetFuncBody<'env>),
    LoadFile(LoadFile<'env>),
    ResolveNamespaceItems(ResolveNamespaceItems<'env>),
    ResolveNamespace(ResolveNamespace<'env>),
    ResolveWhen(ResolveWhen<'env>),
    EvaluateComptime(EvaluateComptime<'env>),
    ResolveEvaluation(ResolveEvaluation<'env>),
    ResolveFunction(ResolveFunction<'env>),
    ResolveFunctionHead(ResolveFunctionHead<'env>),
    ResolveFunctionBody(ResolveFunctionBody<'env>),
    BuildIr(BuildIr<'env>),
    LowerFunctionHead(LowerFunctionHead<'env>),
}

#[derive(Debug, PartialEq, Eq, Hash)]
#[enum_dispatch(Spawnable)]
pub enum Request<'env> {
    Diverge(Diverge),
    Print(Print<'env>),
    FindType(FindType<'env>),
    //GetTypeHead(GetTypeHead<'env>),
    //GetTypeBody(GetTypeBody<'env>),
    ResolveType(ResolveType<'env>),
    // ResolveTypeKeepAliases(ResolveTypeKeepAliases<'env>),
    // ResolveTypeArg(ResolveTypeArg<'env>),
    // GetFuncHead(GetFuncHead<'env>),
    // GetFuncBody(GetFuncBody<'env>),
    LoadFile(LoadFile<'env>),
    ResolveNamespaceItems(ResolveNamespaceItems<'env>),
    ResolveNamespace(ResolveNamespace<'env>),
    ResolveWhen(ResolveWhen<'env>),
    ResolveEvaluation(ResolveEvaluation<'env>),
    ResolveFunction(ResolveFunction<'env>),
    ResolveFunctionHead(ResolveFunctionHead<'env>),
    ResolveFunctionBody(ResolveFunctionBody<'env>),
    BuildIr(BuildIr<'env>),
    LowerFunctionHead(LowerFunctionHead<'env>),
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
