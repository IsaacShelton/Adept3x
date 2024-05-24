mod from_iterator;

use self::from_iterator::TextStreamFromIterator;
use super::TextStream;
use crate::source_file_cache::SourceFileCacheKey;

pub trait IntoTextStream {
    fn into_text_stream(self, source_key: SourceFileCacheKey) -> impl TextStream;
}

impl<I> IntoTextStream for I
where
    I: Iterator<Item = char>,
{
    fn into_text_stream(self, source_key: SourceFileCacheKey) -> impl TextStream {
        TextStreamFromIterator::new(self, source_key)
    }
}
