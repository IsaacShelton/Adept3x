use crate::{lexical_utils::IsCharacter, look_ahead::LookAhead};

/*
   Handles splicing together of physical source lines to form logical source lines.

   This encompasses Translation Phase 2 of the C standard.

   Each line that ends in a backslash will be joined with the following line
*/

#[derive(Clone, Debug)]
pub struct Line {
    pub content: String,
    pub line_number: usize,
}

pub struct LineSplicer<I>
where
    I: Iterator<Item = char>,
{
    chars: LookAhead<I>,
    current_line: String,
    next_line_number: usize,
    newlines: usize,
}

impl<I: Iterator<Item = char>> LineSplicer<I> {
    pub fn new(iterator: I) -> Self {
        Self {
            chars: LookAhead::new(iterator),
            current_line: String::new(),
            next_line_number: 1,
            newlines: 0,
        }
    }
}

impl<I: Iterator<Item = char>> Iterator for LineSplicer<I> {
    type Item = Line;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.chars.next() {
                Some('\n') => {
                    // Remember line number of current line
                    let line_number = self.next_line_number;

                    // Advance line number for next line
                    self.newlines += 1;
                    self.next_line_number = self.newlines;

                    return Some(Line {
                        content: std::mem::take(&mut self.current_line),
                        line_number,
                    });
                }
                Some('\\') if self.chars.peek().is_character('\n') => {
                    // Join current line with next line, since it ended in a backslash
                    self.newlines += 1;
                    self.chars.next();
                }
                Some(c) => {
                    self.current_line.push(c);
                }
                None if !self.current_line.is_empty() => {
                    return Some(Line {
                        content: std::mem::take(&mut self.current_line),
                        line_number: self.next_line_number,
                    });
                }
                None => return None,
            }
        }
    }
}
