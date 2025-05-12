use crate::Execution;

pub trait SpawnExecution<'env> {
    fn spawn_execution(&self) -> Execution<'env>;
}
