mod adapter;
mod as_iter;
mod is_end;
mod peekable;
mod peeker;

pub use adapter::Adapter;
pub use as_iter::AsIter;
pub use is_end::IsEnd;
pub use peekable::Peekable;
pub use peeker::Peeker;

pub trait InfiniteIterator {
    type Item: IsEnd;
    fn next(&mut self) -> Self::Item;
}

impl<T: IsEnd, II: InfiniteIterator<Item = T>> InfiniteIterator for &mut II {
    type Item = T;

    fn next(&mut self) -> Self::Item {
        (**self).next()
    }
}
