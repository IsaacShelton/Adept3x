use super::{into_text::TextPeeker, Character, Text};

pub trait TextStream {
    fn next(&mut self) -> Character;

    fn into_text(self) -> impl Text
    where
        Self: Sized,
    {
        TextPeeker::new(self)
    }
}
