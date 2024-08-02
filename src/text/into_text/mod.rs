mod peeker;

pub use self::peeker::TextPeeker;
use super::{IntoTextStream, Text};
use crate::source_files::SourceFileKey;

pub trait IntoText {
    fn into_text(self, source_file_id: SourceFileKey) -> impl Text;
}

impl<S: IntoTextStream> IntoText for S {
    fn into_text(self, source_file_id: SourceFileKey) -> impl Text {
        TextPeeker::new(self.into_text_stream(source_file_id))
    }
}
