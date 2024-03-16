use crate::{
    look_ahead::LookAhead,
    repeating_last::RepeatingLast,
    source_file_cache::{SourceFileCache, SourceFileCacheKey},
    token::{Token, TokenKind},
};
use std::borrow::Borrow;

pub struct Input<'a, I: Iterator<Item = Token>> {
    source_file_cache: &'a SourceFileCache,
    iterator: LookAhead<RepeatingLast<I>>,
    key: SourceFileCacheKey,
}

impl<'a, I> Input<'a, I>
where
    I: Iterator<Item = Token>,
{
    pub fn new(
        iterator: I,
        source_file_cache: &'a SourceFileCache,
        key: SourceFileCacheKey,
    ) -> Self {
        Self {
            iterator: LookAhead::new(RepeatingLast::new(iterator)),
            source_file_cache,
            key,
        }
    }

    pub fn peek(&mut self) -> &Token {
        self.iterator.peek_nth(0).unwrap()
    }

    pub fn peek_nth(&mut self, n: usize) -> &Token {
        self.iterator.peek_nth(n).unwrap()
    }

    pub fn peek_is(&mut self, token: impl Borrow<TokenKind>) -> bool {
        self.peek().kind == *token.borrow()
    }

    pub fn peek_is_or_eof(&mut self, token: impl Borrow<TokenKind>) -> bool {
        let next = &self.peek().kind;
        next == token.borrow() || next.is_end_of_file()
    }

    pub fn advance(&mut self) -> Token {
        self.iterator.next().unwrap()
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
