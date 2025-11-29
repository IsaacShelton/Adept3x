use crate::Character;
use infinite_iterator::InfiniteIterator;
use line_column::{LineColumn, Location};

pub struct CharacterInfiniteIterator<I: Iterator<Item = char>> {
    iterator: LineColumn<I>,
    last_location: Location,
}

impl<I> CharacterInfiniteIterator<I>
where
    I: Iterator<Item = char>,
{
    pub fn new(iterator: I) -> Self {
        Self {
            iterator: LineColumn::new(iterator),
            last_location: Location { line: 1, column: 1 },
        }
    }
}

impl<I> InfiniteIterator for CharacterInfiniteIterator<I>
where
    I: Iterator<Item = char>,
{
    type Item = Character;

    fn next(&mut self) -> Self::Item {
        match self.iterator.next() {
            Some((character, location)) => {
                self.last_location = location;
                Character::At(character, location)
            }
            None => Character::End(self.last_location),
        }
    }
}

unsafe impl<I> Send for CharacterInfiniteIterator<I> where I: Iterator<Item = char> + Send {}
