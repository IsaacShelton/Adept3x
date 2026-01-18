use document::Document;
use text_edit::TextEditOrFullUtf16;
use util_iter_coproduct::IteratorCoproduct2;

#[derive(Debug)]
pub enum FileBytes {
    Document(Document),
    Text(String),
}

impl FileBytes {
    pub fn chars(&self) -> impl Iterator<Item = char> {
        match self {
            Self::Document(document) => IteratorCoproduct2::Left(document.chars()),
            Self::Text(text) => IteratorCoproduct2::Right(text.chars()),
        }
    }

    pub fn after_edits(&self, edits: impl Iterator<Item = TextEditOrFullUtf16>) -> Self {
        let mut document = match self {
            FileBytes::Document(document) => (*document).clone(),
            FileBytes::Text(text) => Document::new(text.clone().into()),
        };

        for text_edit in edits {
            document.apply_utf16_text_edit_or_full(text_edit);
        }

        FileBytes::Document(document)
    }
}
