use crate::{Character, Text};
use infinite_iterator::InfiniteIterator;
use std::collections::VecDeque;

pub struct CharacterPeeker<I, S: Copy>
where
    I: InfiniteIterator<Item = Character<S>>,
{
    stream: I,
    queue: VecDeque<(char, S)>,
}

impl<I, S: Copy> CharacterPeeker<I, S>
where
    I: InfiniteIterator<Item = Character<S>>,
{
    pub fn new(stream: I) -> Self {
        Self {
            stream,
            queue: VecDeque::new(),
        }
    }
}

impl<I, S> InfiniteIterator for CharacterPeeker<I, S>
where
    I: InfiniteIterator<Item = Character<S>>,
    S: Copy,
{
    type Item = Character<S>;

    fn next(&mut self) -> Self::Item {
        self.queue
            .pop_front()
            .map(|(c, source)| Character::At(c, source))
            .unwrap_or_else(|| self.stream.next())
    }
}

impl<I, S> Text<S> for CharacterPeeker<I, S>
where
    I: InfiniteIterator<Item = Character<S>>,
    S: Copy,
{
    fn peek_nth(&mut self, n: usize) -> I::Item {
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
