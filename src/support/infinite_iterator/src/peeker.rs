use crate::{InfiniteIterator, InfiniteIteratorEnd, InfinitePeekable};
use std::{collections::VecDeque, mem::MaybeUninit};

pub struct Peeker<I: InfiniteIterator> {
    stream: I,
    queue: VecDeque<I::Item>,
}

impl<I: InfiniteIterator> Peeker<I> {
    pub fn new(stream: I) -> Self {
        Self {
            stream,
            queue: VecDeque::with_capacity(8),
        }
    }
}

impl<I: InfiniteIterator> InfiniteIterator for Peeker<I> {
    type Item = I::Item;

    fn next(&mut self) -> Self::Item {
        self.queue.pop_front().unwrap_or_else(|| self.stream.next())
    }
}

unsafe impl<I: InfiniteIterator + Send> Send for Peeker<I> {}

impl<T, I> InfinitePeekable<T> for Peeker<I>
where
    T: InfiniteIteratorEnd,
    I: InfiniteIterator<Item = T>,
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

        let mut array: [MaybeUninit<&T>; N] = [const { MaybeUninit::uninit() }; N];
        for i in 0..N {
            array[i].write(&self.queue[i]);
        }

        // SAFETY: We have initialized all elements
        // Why does Rust not have a stablized method to do this?
        unsafe { MaybeUninit::array_assume_init(array) }
    }
}
