use crate::{Character, Text, TextStream};
use source_files::Source;
use std::collections::VecDeque;

pub struct TextPeeker<S: TextStream> {
    stream: S,
    queue: VecDeque<(char, Source)>,
}

impl<S: TextStream> TextPeeker<S> {
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            queue: VecDeque::new(),
        }
    }
}

impl<S: TextStream> TextStream for TextPeeker<S> {
    fn next(&mut self) -> Character {
        self.queue
            .pop_front()
            .map(|(c, source)| Character::At(c, source))
            .unwrap_or_else(|| self.stream.next())
    }
}

impl<S: TextStream> Text for TextPeeker<S> {
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
