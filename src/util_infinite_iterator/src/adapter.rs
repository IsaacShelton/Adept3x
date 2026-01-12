use crate::{InfiniteIterator, IsEnd};
use std::iter::Fuse;

pub struct Adapter<T, I>
where
    T: Clone + IsEnd,
    I: Iterator,
{
    iterator: Fuse<I>,
    end: T,
}

impl<T, I> Adapter<T, I>
where
    T: Clone + IsEnd,
    I: Iterator,
{
    pub fn new(iterator: I, end: T) -> Self {
        Self {
            iterator: iterator.fuse(),
            end,
        }
    }
}

impl<T, I> InfiniteIterator for Adapter<T, I>
where
    T: Clone + IsEnd,
    I: Iterator<Item = T>,
{
    type Item = T;

    fn next(&mut self) -> Self::Item {
        self.iterator.next().unwrap_or(self.end.clone())
    }
}

unsafe impl<T, I> Send for Adapter<T, I>
where
    T: Clone + Send + IsEnd,
    I: Iterator + Send,
{
}
