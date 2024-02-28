use std::{collections::VecDeque, iter::Fuse};

pub struct LookAhead<I>
where
    I: Iterator,
{
    iterator: Fuse<I>,
    buffer: VecDeque<I::Item>,
}

impl<I> LookAhead<I>
where
    I: Iterator,
{
    pub fn new(iterator: I) -> Self {
        Self {
            iterator: iterator.fuse(),
            buffer: VecDeque::new(),
        }
    }

    #[allow(dead_code)]
    pub fn peek<'a>(&'a mut self) -> Option<&'a <Self as Iterator>::Item> {
        self.peek_nth(0)
    }

    #[allow(dead_code)]
    pub fn peek_nth<'a>(&'a mut self, index: usize) -> Option<&'a <Self as Iterator>::Item> {
        while self.buffer.len() <= index {
            if let Some(value) = self.iterator.next() {
                self.buffer.push_back(value);
            } else {
                return None;
            }
        }

        self.buffer.get(index)
    }
}

impl<I: Iterator> Iterator for LookAhead<I>
where
    I: Iterator,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.buffer.pop_front().or_else(|| self.iterator.next())
    }
}
