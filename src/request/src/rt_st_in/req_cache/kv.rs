use crate::{Pf, TaskStatus};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Kv<'e, P: Pf> {
    pub(crate) inner: HashMap<<P as Pf>::Req<'e>, Option<TaskStatus<'e, P>>>,
}
