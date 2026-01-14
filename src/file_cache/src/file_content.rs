use document::Document;
use util_iter_coproduct::IteratorCoproduct2;

pub enum FileContent {
    Document(Document),
    Text(String),
}

impl FileContent {
    pub fn chars(&self) -> impl Iterator<Item = char> {
        match self {
            Self::Document(document) => IteratorCoproduct2::Left(document.chars()),
            Self::Text(text) => IteratorCoproduct2::Right(text.chars()),
        }
    }
}
