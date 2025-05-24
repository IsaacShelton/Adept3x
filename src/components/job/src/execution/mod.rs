mod build_asg;
mod diverge;
mod estimate_decl_scope;
mod estimate_type_heads;
mod find_type;
mod find_type_in_decl_set;
mod find_type_in_estimated;
mod get_type_body;
mod get_type_head;
mod print;
mod resolve_type;
mod resolve_type_arg;

use crate::{Artifact, Continuation, ExecutionCtx, Executor, TaskRef, UnwrapFrom};
pub use build_asg::BuildAsg;
pub use diverge::Diverge;
use enum_dispatch::enum_dispatch;
pub use estimate_decl_scope::EstimateDeclScope;
use estimate_type_heads::EstimateTypeHeads;
pub use find_type::FindType;
use find_type_in_decl_set::FindTypeInDeclSet;
pub use find_type_in_estimated::FindTypeInEstimated;
pub use get_type_body::GetTypeBody;
pub use get_type_head::GetTypeHead;
pub use print::Print;
pub use resolve_type::ResolveType;
pub use resolve_type_arg::*;

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
    Diverge(Diverge),
    Print(Print<'env>),
    BuildAsg(BuildAsg<'env>),
    EstimateDeclScope(EstimateDeclScope<'env>),
    GetTypeHead(GetTypeHead<'env>),
    EstimateTypeHeads(EstimateTypeHeads<'env>),
    FindTypeInEstimated(FindTypeInEstimated<'env>),
    FindTypeInDeclSet(FindTypeInDeclSet<'env>),
    FindType(FindType<'env>),
    GetTyp1Body(GetTypeBody<'env>),
    ResolveType(ResolveType<'env>),
    ResolveTypeArg(ResolveTypeArg<'env>),
}

#[derive(Debug, PartialEq, Eq, Hash)]
#[enum_dispatch(Spawnable)]
pub enum Request<'env> {
    Diverge(Diverge),
    Print(Print<'env>),
    EstimateDeclScope(EstimateDeclScope<'env>),
    GetTypeHead(GetTypeHead<'env>),
    EstimateTypeHeads(EstimateTypeHeads<'env>),
    FindTypeInEstimated(FindTypeInEstimated<'env>),
    FindTypeInDeclSet(FindTypeInDeclSet<'env>),
    FindType(FindType<'env>),
    GetTypeBody(GetTypeBody<'env>),
    ResolveType(ResolveType<'env>),
    ResolveTypeArg(ResolveTypeArg<'env>),
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
