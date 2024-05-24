use crate::inflow::{Inflow, InflowEnd, InflowStream};
use std::collections::VecDeque;

pub struct InflowPeeker<S: InflowStream> {
    stream: S,
    queue: VecDeque<S::Item>,
}

impl<S: InflowStream> InflowPeeker<S> {
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            queue: VecDeque::new(),
        }
    }
}

impl<S: InflowStream> InflowStream for InflowPeeker<S> {
    type Item = S::Item;

    fn next(&mut self) -> Self::Item {
        self.queue.pop_front().unwrap_or_else(|| self.stream.next())
    }
}

impl<T: InflowEnd, S: InflowStream<Item = T>> Inflow<T> for InflowPeeker<S> {
    fn peek_nth_mut<'a>(&'a mut self, n: usize) -> &'a mut T {
        while self.queue.len() <= n {
            let item = self.stream.next();
            self.queue.push_back(item);
        }

        self.queue.get_mut(n).unwrap()
    }
}
