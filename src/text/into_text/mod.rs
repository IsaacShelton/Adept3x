mod peeker;

use super::{IntoTextStream, Text};
use crate::source_file_cache::SourceFileCacheKey;

pub use self::peeker::TextPeeker;

pub trait IntoText {
    fn into_text(self, source_file_id: SourceFileCacheKey) -> impl Text;
}

impl<S: IntoTextStream> IntoText for S {
    fn into_text(self, source_file_id: SourceFileCacheKey) -> impl Text {
        TextPeeker::new(self.into_text_stream(source_file_id))
    }
}
