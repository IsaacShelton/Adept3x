use crate::Character;
use infinite_iterator::InfiniteIterator;
use line_column::{LineColumn, Location};

pub struct CharacterInfiniteIterator<I: Iterator<Item = char>, S, F: Fn(Location) -> S> {
    iterator: LineColumn<I>,
    last_location: Location,
    to_source: F,
}

impl<I, S, F> CharacterInfiniteIterator<I, S, F>
where
    I: Iterator<Item = char>,
    F: Fn(Location) -> S,
{
    pub fn new(iterator: I, to_source: F) -> Self {
        Self {
            iterator: LineColumn::new(iterator),
            last_location: Location { line: 1, column: 1 },
            to_source,
        }
    }
}

impl<I, S, F> InfiniteIterator for CharacterInfiniteIterator<I, S, F>
where
    I: Iterator<Item = char>,
    F: Fn(Location) -> S,
    S: Copy,
{
    type Item = Character<S>;

    fn next(&mut self) -> Self::Item {
        match self.iterator.next() {
            Some((character, location)) => {
                self.last_location = location;
                Character::At(character, (self.to_source)(location))
            }
            None => Character::End((self.to_source)(self.last_location)),
        }
    }
}

unsafe impl<I, S, F> Send for CharacterInfiniteIterator<I, S, F>
where
    I: Iterator<Item = char> + Send,
    F: Fn(Location) -> S,
{
}
