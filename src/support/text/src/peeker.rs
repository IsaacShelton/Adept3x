use crate::{Character, Text};
use infinite_iterator::InfiniteIterator;
use source_files::Source;
use std::collections::VecDeque;

pub struct TextPeeker<I>
where
    I: InfiniteIterator<Item = Character>,
{
    stream: I,
    queue: VecDeque<(char, Source)>,
}

impl<I> TextPeeker<I>
where
    I: InfiniteIterator<Item = Character>,
{
    pub fn new(stream: I) -> Self {
        Self {
            stream,
            queue: VecDeque::new(),
        }
    }
}

impl<I> InfiniteIterator for TextPeeker<I>
where
    I: InfiniteIterator<Item = Character>,
{
    type Item = Character;

    fn next(&mut self) -> Character {
        self.queue
            .pop_front()
            .map(|(c, source)| Character::At(c, source))
            .unwrap_or_else(|| self.stream.next())
    }
}

impl<I> Text for TextPeeker<I>
where
    I: InfiniteIterator<Item = Character>,
{
    fn peek_nth(&mut self, n: usize) -> Character {
        while self.queue.len() <= n {
            match self.stream.next() {
                Character::At(c, source) => self.queue.push_back((c, source)),
                Character::End(source) => return Character::End(source),
            }
        }

        self.queue
            .get(n)
            .map(|(c, source)| Character::At(*c, *source))
            .unwrap()
    }
}
