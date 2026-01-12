use crate::{InfiniteIterator, IsEnd, Peekable};
use std::collections::VecDeque;

pub struct Peeker<II: InfiniteIterator> {
    stream: II,
    queue: VecDeque<II::Item>,
}

impl<II: InfiniteIterator> Peeker<II> {
    pub fn new(stream: II) -> Self {
        Self {
            stream,
            queue: VecDeque::with_capacity(8),
        }
    }
}

impl<II: InfiniteIterator> InfiniteIterator for Peeker<II> {
    type Item = II::Item;

    fn next(&mut self) -> Self::Item {
        self.queue.pop_front().unwrap_or_else(|| self.stream.next())
    }
}

unsafe impl<II: InfiniteIterator + Send> Send for Peeker<II> {}

impl<T, II> Peekable<T> for Peeker<II>
where
    T: IsEnd,
    II: InfiniteIterator<Item = T>,
{
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

        std::array::from_fn(|i| &self.queue[i])
    }
}
