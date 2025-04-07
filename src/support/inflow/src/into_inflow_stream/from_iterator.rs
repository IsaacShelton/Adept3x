use crate::{InflowEnd, InflowStream};
use std::iter::Fuse;

pub struct InflowStreamFromIterator<T: Clone + InflowEnd, I: Iterator> {
    iterator: Fuse<I>,
    end: T,
}

unsafe impl<T: Clone + InflowEnd, I: Iterator> Send for InflowStreamFromIterator<T, I> {}

impl<T: Clone + InflowEnd, I: Iterator<Item = T>> InflowStreamFromIterator<T, I> {
    pub fn new(iterator: I, end: T) -> Self {
        Self {
            iterator: iterator.fuse(),
            end,
        }
    }
}

impl<T: Clone + InflowEnd, I: Iterator<Item = T>> InflowStream for InflowStreamFromIterator<T, I> {
    type Item = T;

    fn next(&mut self) -> Self::Item {
        self.iterator.next().unwrap_or(self.end.clone())
    }
}
