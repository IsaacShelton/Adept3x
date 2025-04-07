#![feature(maybe_uninit_array_assume_init)]

mod end;
mod infinite;
mod iter;
mod peekable;
mod peeker;
mod tools;

pub use end::InfiniteIteratorEnd;
pub use infinite::Infinite;
pub use iter::InfiniteIterator;
pub use peekable::InfinitePeekable;
pub use peeker::Peeker as InfiniteIteratorPeeker;
pub use tools::InfiniteIteratorTools;
