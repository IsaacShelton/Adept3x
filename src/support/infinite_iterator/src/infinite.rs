use crate::{InfiniteIterator, InfiniteIteratorEnd};
use std::iter::Fuse;

pub struct Infinite<T, I>
where
    T: Clone + InfiniteIteratorEnd,
    I: Iterator,
{
    iterator: Fuse<I>,
    end: T,
}

impl<T, I> Infinite<T, I>
where
    T: Clone + InfiniteIteratorEnd,
    I: Iterator,
{
    pub fn new(iterator: I, end: T) -> Self {
        Self {
            iterator: iterator.fuse(),
            end,
        }
    }
}

impl<T, I> InfiniteIterator for Infinite<T, I>
where
    T: Clone + InfiniteIteratorEnd,
    I: Iterator<Item = T>,
{
    type Item = T;

    fn next(&mut self) -> Self::Item {
        self.iterator.next().unwrap_or(self.end.clone())
    }
}

unsafe impl<T, I> Send for Infinite<T, I>
where
    T: Clone + Send + InfiniteIteratorEnd,
    I: Iterator + Send,
{
}
