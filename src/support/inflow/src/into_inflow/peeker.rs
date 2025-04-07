use crate::{Inflow, InflowEnd, InflowStream};
use std::{collections::VecDeque, mem::MaybeUninit};

pub struct InflowPeeker<S: InflowStream> {
    stream: S,
    queue: VecDeque<S::Item>,
}

unsafe impl<S: InflowStream + Send> Send for InflowPeeker<S> {}

impl<S: InflowStream> InflowPeeker<S> {
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            queue: VecDeque::with_capacity(8),
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
    fn un_next(&mut self, item: Self::Item) {
        self.queue.push_front(item);
    }

    fn peek_nth_mut(&mut self, n: usize) -> &mut T {
        while self.queue.len() <= n {
            let item = self.stream.next();
            self.queue.push_back(item);
        }

        self.queue.get_mut(n).unwrap()
    }

    fn peek_n<const N: usize>(&mut self) -> [&T; N] {
        while self.queue.len() <= N {
            let item = self.stream.next();
            self.queue.push_back(item);
        }

        let mut array: [MaybeUninit<&T>; N] = [const { MaybeUninit::uninit() }; N];
        for i in 0..N {
            array[i].write(&self.queue[i]);
        }

        // SAFETY: We have initialized all elements
        // Why does Rust not have a stablized method to do this?
        unsafe { MaybeUninit::array_assume_init(array) }
    }
}
