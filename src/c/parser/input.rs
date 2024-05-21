use crate::{
    c::token::{CToken, CTokenKind},
    look_ahead::LookAhead,
    repeating_last::RepeatingLast,
    source_file_cache::{SourceFileCache, SourceFileCacheKey},
};
use std::{borrow::Borrow, collections::VecDeque};

pub struct Input<'a, I: Iterator<Item = CToken> + Clone> {
    source_file_cache: &'a SourceFileCache,
    stack: VecDeque<LookAhead<RepeatingLast<I>>>,
    key: SourceFileCacheKey,
}

impl<'a, I> Input<'a, I>
where
    I: Iterator<Item = CToken> + Clone,
{
    pub fn new(
        iterator: I,
        source_file_cache: &'a SourceFileCache,
        key: SourceFileCacheKey,
    ) -> Self {
        let mut stack = VecDeque::with_capacity(4);
        stack.push_front(LookAhead::new(RepeatingLast::new(iterator)));

        Self {
            stack,
            source_file_cache,
            key,
        }
    }

    pub fn peek(&mut self) -> &CToken {
        self.stack.front_mut().unwrap().peek_nth(0).unwrap()
    }

    pub fn peek_nth(&mut self, n: usize) -> &CToken {
        self.stack.front_mut().unwrap().peek_nth(n).unwrap()
    }

    pub fn peek_n(&mut self, n: usize) -> &[CToken] {
        self.stack.front_mut().unwrap().peek_n(n)
    }

    pub fn peek_is(&mut self, token: impl Borrow<CTokenKind>) -> bool {
        self.peek().kind == *token.borrow()
    }

    pub fn peek_is_or_eof(&mut self, token: impl Borrow<CTokenKind>) -> bool {
        let next = &self.peek().kind;
        next == token.borrow() || next.is_end_of_file()
    }

    pub fn advance(&mut self) -> CToken {
        self.stack.front_mut().unwrap().next().unwrap()
    }

    pub fn speculate(&mut self) {
        self.stack.push_front(self.stack.front().unwrap().clone());
    }

    pub fn backtrack(&mut self) {
        self.stack.pop_front();
        assert!(!self.stack.is_empty());
    }

    pub fn success(&mut self) {
        // Indicates that a speculation was successful,
        // so we can remove the backtrack point
        let success = self.stack.pop_front().unwrap();
        let _ = self.stack.pop_front().unwrap();
        self.stack.push_front(success);
    }

    pub fn filename(&self) -> &str {
        self.source_file_cache.get(self.key).filename()
    }

    pub fn key(&self) -> SourceFileCacheKey {
        self.key
    }

    pub fn source_file_cache(&self) -> &'a SourceFileCache {
        self.source_file_cache
    }
}
