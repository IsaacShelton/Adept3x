mod from_iterator;

use self::from_iterator::TextStreamFromIterator;
use super::TextStream;
use source_files::SourceFileKey;

pub trait IntoTextStream {
    fn into_text_stream(self, file_key: SourceFileKey) -> impl TextStream + Send;
}

impl<I> IntoTextStream for I
where
    I: Iterator<Item = char> + Send,
{
    fn into_text_stream(self, file_key: SourceFileKey) -> impl TextStream + Send {
        TextStreamFromIterator::new(self, file_key)
    }
}
