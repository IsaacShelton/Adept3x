use crate::TaskRef;

pub trait Prereqs<'env> {
    fn prereqs(&self) -> Vec<TaskRef<'env>>;
}
