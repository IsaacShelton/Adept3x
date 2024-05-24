use crate::{
    ast::Source,
    line_column::{LineColumn, Location},
    source_file_cache::SourceFileCacheKey,
    text::{Character, TextStream},
};

pub struct TextStreamFromIterator<I: Iterator<Item = char>> {
    iterator: LineColumn<I>,
    source_key: SourceFileCacheKey,
    last_location: Location,
}

impl<I: Iterator<Item = char>> TextStreamFromIterator<I> {
    pub fn new(iterator: I, source_key: SourceFileCacheKey) -> Self {
        Self {
            iterator: LineColumn::new(iterator),
            source_key,
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
                        key: self.source_key,
                        location,
                    },
                )
            }
            None => Character::End(Source {
                key: self.source_key,
                location: self.last_location,
            }),
        }
    }
}
