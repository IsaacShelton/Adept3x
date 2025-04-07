use crate::{InfiniteIterator, InfiniteIteratorEnd};

pub trait InfinitePeekable<T>: InfiniteIterator<Item = T> {
    fn un_next(&mut self, item: Self::Item);

    fn peek_nth_mut(&mut self, n: usize) -> &mut T;
    fn peek_n<const N: usize>(&mut self) -> [&T; N];

    fn peek_nth(&mut self, n: usize) -> &T {
        &*self.peek_nth_mut(n)
    }

    fn peek(&mut self) -> &T {
        self.peek_nth(0)
    }

    fn peek_mut(&mut self) -> &mut T {
        self.peek_nth_mut(0)
    }
}

impl<T: InfiniteIteratorEnd, I: InfinitePeekable<T>> InfinitePeekable<T> for &mut I {
    fn un_next(&mut self, item: Self::Item) {
        (**self).un_next(item)
    }

    fn peek_nth_mut(&mut self, n: usize) -> &mut T {
        (**self).peek_nth_mut(n)
    }

    fn peek_n<const N: usize>(&mut self) -> [&T; N] {
        (**self).peek_n::<N>()
    }
}
