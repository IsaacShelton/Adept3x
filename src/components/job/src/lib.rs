macro_rules! impl_unwrap_from_artifact {
    ($variant:ident, $self_ty:ty) => {
        impl<'env> crate::UnwrapFrom<Artifact<'env>> for $self_ty {
            fn unwrap_from<'a>(from: &'a Artifact<'env>) -> &'a Self {
                match from {
                    Artifact::$variant(value) => value,
                    _ => panic!(),
                }
            }
        }
    };
}

macro_rules! suspend {
    ($self:ident.$field:ident, $task_ref:expr, $ctx:expr) => {{
        let pending = $task_ref;

        $ctx.suspend_on(::std::iter::once(pending.raw_task_ref()));

        Err(Continuation::suspend(Self {
            $field: Some(pending),
            ..$self
        }))
    }};
}

macro_rules! suspend_many {
    ($self:ident.$field:ident, $task_refs:expr, $ctx:expr) => {{
        let pending: Box<[crate::Pending<'env, _>]> = $task_refs;

        $ctx.suspend_on(pending.iter());

        Err(Continuation::suspend(Self {
            $field: Some(pending),
            ..$self
        }))
    }};
}

macro_rules! suspend_many_assoc {
    ($self:ident.$field:ident, $task_refs:expr, $ctx:expr) => {{
        let pending: crate::PendingManyAssoc<'env, _, _> = $task_refs;

        $ctx.suspend_on(pending.iter().map(|(_k, v)| v));

        Err(Continuation::suspend(Self {
            $field: Some(pending),
            ..$self
        }))
    }};
}

macro_rules! suspend_from_subtask {
    ($self:ident, $field:ident, $task_ref:expr, $ctx:expr) => {{
        let pending = $task_ref;

        $ctx.suspend_on(::std::iter::once(pending.raw_task_ref()));
        $self.$field = Some(pending);
        Err(Ok(Continuation::Suspend))
    }};
}

macro_rules! execute_sub_task {
    ($self:ident, $sub_task:expr, $executor:expr, $ctx:expr) => {
        execute_sub_task!($self, $sub_task, $executor, $ctx, ())
    };
    ($self:ident, $sub_task:expr, $executor:expr, $ctx:expr, $user_data:expr) => {{
        match $sub_task.execute_sub_task($executor, $ctx, $user_data) {
            Ok(ok) => ok,
            Err(e) => {
                return Err(e.map(|into_continuation| into_continuation($self.into()))?);
            }
        }
    }};
}

mod allocator;
mod artifact;
mod cfg;
mod conform;
mod continuation;
mod execution;
mod execution_ctx;
mod executor;
mod io;
mod main_executor;
mod module_graph;
mod pending;
mod poly;
mod repr;
mod sub_task;
mod suspend_condition;
mod task;
mod task_state;
mod top_n;
mod truth;
mod typed_cfg;
mod unify;
mod unwrap_from;
mod waiting_count;
mod worker;

pub use allocator::*;
pub use artifact::*;
pub use continuation::*;
pub use execution::*;
pub use execution_ctx::*;
pub use executor::*;
pub use main_executor::*;
pub use pending::*;
pub use poly::*;
pub use suspend_condition::*;
pub use task::*;
pub use task_state::*;
pub use top_n::*;
pub use truth::*;
pub use typed_cfg::*;
pub use unwrap_from::*;
pub use waiting_count::*;
pub use worker::*;
