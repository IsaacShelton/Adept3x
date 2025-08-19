mod canonicalize;
mod diverge;
mod find_type;
mod find_type_in_decl_set;
mod get_func_body;
mod get_func_head;
mod get_type_body;
mod get_type_head;
mod main;
mod print;
mod resolve_type;
mod resolve_type_arg;
mod resolve_type_keep_aliases;
mod semantic;

use crate::{
    Artifact, Continuation, ExecutionCtx, Executor, TaskRef, UnwrapFrom, execution::main::LoadFile,
};
pub use canonicalize::canonicalize_or_error;
pub use diverge::Diverge;
use enum_dispatch::enum_dispatch;
pub use find_type::FindType;
use find_type_in_decl_set::FindTypeInDeclSet;
pub use get_func_body::*;
pub use get_func_head::*;
pub use get_type_body::GetTypeBody;
pub use get_type_head::GetTypeHead;
pub use main::Main;
pub use print::Print;
pub use resolve_type::ResolveType;
pub use resolve_type_arg::*;
pub use resolve_type_keep_aliases::*;
pub use semantic::*;

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
    GetTypeHead(GetTypeHead<'env>),
    FindTypeInDeclSet(FindTypeInDeclSet<'env>),
    FindType(FindType<'env>),
    GetTypeBody(GetTypeBody<'env>),
    ResolveType(ResolveType<'env>),
    ResolveTypeKeepAliases(ResolveTypeKeepAliases<'env>),
    ResolveTypeArg(ResolveTypeArg<'env>),
    GetFuncHead(GetFuncHead<'env>),
    GetFuncBody(GetFuncBody<'env>),
    LoadFile(LoadFile<'env>),
    ResolveNamespaceItems(ResolveNamespaceItems<'env>),
    ResolveNamespace(ResolveNamespace<'env>),
    ResolveWhen(ResolveWhen<'env>),
    EvaluateComptime(EvaluateComptime<'env>),
    ResolveEvaluation(ResolveEvaluation<'env>),
    ResolveFunction(ResolveFunction<'env>),
    ResolveFunctionHead(ResolveFunctionHead<'env>),
}

#[derive(Debug, PartialEq, Eq, Hash)]
#[enum_dispatch(Spawnable)]
pub enum Request<'env> {
    Diverge(Diverge),
    Print(Print<'env>),
    GetTypeHead(GetTypeHead<'env>),
    FindTypeInDeclSet(FindTypeInDeclSet<'env>),
    FindType(FindType<'env>),
    GetTypeBody(GetTypeBody<'env>),
    ResolveType(ResolveType<'env>),
    ResolveTypeKeepAliases(ResolveTypeKeepAliases<'env>),
    ResolveTypeArg(ResolveTypeArg<'env>),
    GetFuncHead(GetFuncHead<'env>),
    GetFuncBody(GetFuncBody<'env>),
    LoadFile(LoadFile<'env>),
    ResolveNamespaceItems(ResolveNamespaceItems<'env>),
    ResolveNamespace(ResolveNamespace<'env>),
    ResolveWhen(ResolveWhen<'env>),
    // NOTE: AST expressions are not Hash + PartialEq
    // EvaluateComptime(EvaluateComptime<'env>),
    ResolveEvaluation(ResolveEvaluation<'env>),
    ResolveFunction(ResolveFunction<'env>),
    ResolveFunctionHead(ResolveFunctionHead<'env>),
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
