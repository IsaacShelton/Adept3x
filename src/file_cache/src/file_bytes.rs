use document::Document;
use text_edit::TextEditOrFullUtf16;

#[derive(Debug)]
pub enum FileBytes {
    Document(Document),
}

impl FileBytes {
    pub fn chars(&self) -> impl Iterator<Item = char> {
        match self {
            Self::Document(document) => document.chars(),
        }
    }

    pub fn after_edits(&self, edits: impl Iterator<Item = TextEditOrFullUtf16>) -> Self {
        let mut document = match self {
            FileBytes::Document(document) => (*document).clone(),
        };

        for text_edit in edits {
            document.apply_utf16_text_edit_or_full(text_edit);
        }

        FileBytes::Document(document)
    }

    pub fn as_document(&self) -> Option<&Document> {
        match self {
            FileBytes::Document(document) => Some(document),
        }
    }
}
