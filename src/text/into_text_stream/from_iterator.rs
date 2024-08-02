use crate::{
    line_column::{LineColumn, Location},
    source_files::{Source, SourceFileKey},
    text::{Character, TextStream},
};

pub struct TextStreamFromIterator<I: Iterator<Item = char>> {
    iterator: LineColumn<I>,
    file_key: SourceFileKey,
    last_location: Location,
}

impl<I: Iterator<Item = char>> TextStreamFromIterator<I> {
    pub fn new(iterator: I, file_key: SourceFileKey) -> Self {
        Self {
            iterator: LineColumn::new(iterator),
            file_key,
            last_location: Location { line: 1, column: 1 },
        }
    }
}

impl<I: Iterator<Item = char>> TextStream for TextStreamFromIterator<I> {
    fn next(&mut self) -> Character {
        match self.iterator.next() {
            Some((character, location)) => {
                self.last_location = location;
                Character::At(
                    character,
                    Source {
                        key: self.file_key,
                        location,
                    },
                )
            }
            None => Character::End(Source {
                key: self.file_key,
                location: self.last_location,
            }),
        }
    }
}
