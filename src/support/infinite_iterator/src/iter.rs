use crate::InfiniteIteratorEnd;

pub trait InfiniteIterator {
    type Item: InfiniteIteratorEnd;

    fn next(&mut self) -> Self::Item;
}

impl<T: InfiniteIteratorEnd, I: InfiniteIterator<Item = T>> InfiniteIterator for &mut I {
    type Item = T;

    fn next(&mut self) -> Self::Item {
        (**self).next()
    }
}
