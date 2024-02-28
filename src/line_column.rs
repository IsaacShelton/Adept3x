use std::iter::Fuse;

pub struct LineColumn<I: Iterator<Item = char>> {
    iterator: Fuse<I>,
    line: usize,
    column: usize,
    next_line: usize,
    next_column: usize,
}

impl<I> Iterator for LineColumn<I>
where
    I: Iterator<Item = char>,
{
    type Item = I::Item;

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

        character
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

    pub fn friendly_line_number(&self) -> usize {
        self.line + 1
    }

    pub fn friendly_column_number(&self) -> usize {
        self.column + 1
    }

    #[allow(dead_code)]
    pub fn next_friendly_line_number(&self) -> usize {
        self.next_line + 1
    }

    #[allow(dead_code)]
    pub fn next_friendly_column_number(&self) -> usize {
        self.next_column + 1
    }
}
