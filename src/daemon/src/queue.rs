use std::{collections::VecDeque, sync::Mutex};

// TODO
#[derive(Default)]
pub struct PendingQuery;

#[derive(Default)]
pub struct Queue {
    // We can add priorities for this later,
    // but even just running everything at the same time/duration step value
    // is probably fully sufficient.
    _queue: Mutex<VecDeque<PendingQuery>>,
}

impl Queue {
    pub fn next(&self) -> Option<PendingQuery> {
        todo!()
    }
}
