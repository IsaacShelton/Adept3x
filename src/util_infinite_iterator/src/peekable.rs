use crate::{InfiniteIterator, IsEnd};

pub trait Peekable<T>: InfiniteIterator<Item = T> {
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

    fn eat<TSub>(&mut self, predicate: impl FnOnce(T) -> Result<TSub, T>) -> Option<TSub> {
        match predicate(self.next()) {
            Ok(sub) => Some(sub),
            Err(keep) => {
                self.un_next(keep);
                None
            }
        }
    }

    fn peek_skipping<'a>(
        &'a mut self,
        start_index: usize,
        should_skip: impl Fn(&T) -> bool,
    ) -> (&'a T, usize) {
        let new_index = self.peek_skipping_via_index(start_index, should_skip);
        (self.peek_nth(new_index), new_index + 1)
    }

    fn peek_skipping_via_index<'a>(
        &'a mut self,
        start_index: usize,
        should_skip: impl Fn(&T) -> bool,
    ) -> usize {
        let mut index = start_index;

        loop {
            let value = self.peek_nth(index);

            if !(should_skip)(value) {
                return index;
            }

            index += 1;
        }
    }
}

impl<T: IsEnd, II: Peekable<T>> Peekable<T> for &mut II {
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
