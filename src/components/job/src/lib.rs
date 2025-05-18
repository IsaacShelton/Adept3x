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

mod artifact;
mod continuation;
mod execution;
mod executor;
mod main_executor;
mod pending;
mod repr;
mod suspend_condition;
mod task;
mod task_state;
mod truth;
mod unwrap_from;
mod waiting_count;
mod worker;

pub use artifact::*;
pub use continuation::*;
pub use execution::*;
pub use executor::*;
pub use main_executor::*;
pub use pending::*;
pub use suspend_condition::*;
pub use task::*;
pub use task_state::*;
pub use truth::*;
pub use unwrap_from::*;
pub use waiting_count::*;
pub use worker::*;
