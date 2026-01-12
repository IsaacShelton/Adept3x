use crate::{InfiniteIterator, IsEnd};

pub trait AsIter<T>: InfiniteIterator<Item = T>
where
    T: IsEnd,
{
    fn as_iter(&mut self, keep_end: bool) -> impl Iterator<Item = T>;
}

impl<T, II> AsIter<T> for II
where
    T: IsEnd,
    II: InfiniteIterator<Item = T>,
{
    fn as_iter(&mut self, keep_end: bool) -> impl Iterator<Item = T> {
        Iter {
            wrapped: self,
            return_end: keep_end,
        }
    }
}

pub struct Iter<'a, T: IsEnd, II: InfiniteIterator<Item = T> + ?Sized> {
    wrapped: &'a mut II,
    return_end: bool,
}

impl<'a, T: IsEnd, II: InfiniteIterator<Item = T>> Iterator for Iter<'a, T, II> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.wrapped.next();

        if !item.is_end() {
            return Some(item);
        }

        if self.return_end {
            self.return_end = false;
            return Some(item);
        }

        None
    }
}
