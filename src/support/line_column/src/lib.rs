use std::iter::{Fuse, FusedIterator};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Location {
    pub line: u32,
    pub column: u32,
}

impl PartialOrd for Location {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Location {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.line
            .cmp(&other.line)
            .then_with(|| self.column.cmp(&other.column))
    }
}

unsafe impl<I> Send for LineColumn<I> where I: Iterator<Item = char> + Send {}

pub struct LineColumn<I: Iterator<Item = char>> {
    iterator: Fuse<I>,
    line: u32,
    column: u32,
    next_line: u32,
    next_column: u32,
}

impl Location {
    pub fn new(line: u32, column: u32) -> Self {
        Self { line, column }
    }

    pub fn shift_column(&self, amount: u32) -> Self {
        Self {
            line: self.line,
            column: self.column + amount,
        }
    }
}

impl<I: Iterator<Item = char>> FusedIterator for LineColumn<I> {}

impl<I> Iterator for LineColumn<I>
where
    I: Iterator<Item = char>,
{
    type Item = (I::Item, Location);

    fn next(&mut self) -> Option<Self::Item> {
        let character = self.iterator.next();

        self.line = self.next_line;
        self.column = self.next_column;

        match character {
            Some('\n') => {
                self.next_line += 1;
                self.next_column = 0;
            }
            Some(_) => {
                self.next_column += 1;
            }
            None => (),
        }

        character.map(|character| (character, self.friendly_location()))
    }
}

impl<I> LineColumn<I>
where
    I: Iterator<Item = char>,
{
    pub fn new(iterator: I) -> Self {
        Self {
            iterator: iterator.fuse(),
            line: 0,
            column: 0,
            next_line: 0,
            next_column: 0,
        }
    }

    pub fn friendly_location(&self) -> Location {
        Location {
            line: self.line + 1,
            column: self.column + 1,
        }
    }

    #[allow(dead_code)]
    pub fn next_friendly_location(&self) -> Location {
        Location {
            line: self.next_line + 1,
            column: self.next_column + 1,
        }
    }
}
