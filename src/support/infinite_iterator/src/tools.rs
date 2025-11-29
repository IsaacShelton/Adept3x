use crate::{InfiniteIterator, InfiniteIteratorEnd};

pub trait InfiniteIteratorTools<T>: InfiniteIterator<Item = T>
where
    T: InfiniteIteratorEnd,
{
    fn collect_vec(&mut self, keep_end: bool) -> Vec<T> {
        let mut collected = vec![];

        loop {
            let item = self.next();

            if item.is_end() {
                if keep_end {
                    collected.push(item);
                }
                return collected;
            }

            collected.push(item);
        }
    }
}

impl<T, I> InfiniteIteratorTools<T> for I
where
    T: InfiniteIteratorEnd,
    I: InfiniteIterator<Item = T>,
{
}
