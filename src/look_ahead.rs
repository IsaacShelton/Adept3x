use derive_more::Deref;
use std::collections::VecDeque;

#[derive(Deref)]
pub struct LookAhead<I>
where
    I: Iterator,
{
    #[deref]
    iterator: I,

    buffer: VecDeque<I::Item>,
}

impl<I> LookAhead<I>
where
    I: Iterator,
{
    pub fn new(iterator: I) -> Self {
        Self {
            iterator,
            buffer: VecDeque::new(),
        }
    }

    pub fn peek<'a>(&'a mut self) -> Option<&'a I::Item> {
        self.peek_nth(0)
    }

    pub fn peek_n<'a>(&'a mut self, count: usize) -> &'a [I::Item] {
        while self.buffer.len() <= count {
            if let Some(value) = self.iterator.next() {
                self.buffer.push_back(value);
            } else {
                break;
            }
        }

        // TODO: Find better solution
        &self.buffer.make_contiguous()[0..count]
    }

    pub fn peek_nth<'a>(&'a mut self, index: usize) -> Option<&'a I::Item> {
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
