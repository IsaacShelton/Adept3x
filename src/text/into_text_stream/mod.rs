mod from_iterator;

use self::from_iterator::TextStreamFromIterator;
use super::TextStream;
use crate::source_files::SourceFileKey;

pub trait IntoTextStream {
    fn into_text_stream(self, file_key: SourceFileKey) -> impl TextStream;
}

impl<I> IntoTextStream for I
where
    I: Iterator<Item = char>,
{
    fn into_text_stream(self, file_key: SourceFileKey) -> impl TextStream {
        TextStreamFromIterator::new(self, file_key)
    }
}
