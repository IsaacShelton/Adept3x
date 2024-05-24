use std::borrow::Borrow;

use crate::{
    c::token::{CToken, CTokenKind},
    source_file_cache::{SourceFileCache, SourceFileCacheKey},
};

pub struct Input<'a> {
    source_file_cache: &'a SourceFileCache,
    key: SourceFileCacheKey,
    tokens: Vec<CToken>,
    stack: Vec<usize>,
}

impl<'a> Input<'a> {
    pub fn new(
        tokens: Vec<CToken>,
        source_file_cache: &'a SourceFileCache,
        key: SourceFileCacheKey,
    ) -> Self {
        assert!(tokens.last().map_or(false, |token| token.is_end_of_file()));

        Self {
            source_file_cache,
            tokens,
            key,
            stack: vec![0],
        }
    }

    pub fn filename(&self) -> &str {
        self.source_file_cache.get(self.key).filename()
    }

    pub fn source_file_cache(&self) -> &'a SourceFileCache {
        self.source_file_cache
    }

    pub fn key(&self) -> SourceFileCacheKey {
        self.key
    }

    pub fn eof(&self) -> &CToken {
        self.tokens.last().unwrap()
    }

    pub fn peek(&self) -> &CToken {
        self.tokens
            .get(self.stack.last().unwrap() + 1)
            .unwrap_or_else(|| self.eof())
    }

    pub fn peek_nth(&self, n: usize) -> &CToken {
        self.tokens
            .get(self.stack.last().unwrap() + 1 + n)
            .unwrap_or_else(|| self.eof())
    }

    pub fn peek_n(&self, n: usize) -> &[CToken] {
        let start = *self.stack.last().unwrap();
        let end = (start + 1 + n).min(self.tokens.len());
        &self.tokens[start..end]
    }

    pub fn peek_is(&mut self, token: impl Borrow<CTokenKind>) -> bool {
        self.peek().kind == *token.borrow()
    }

    pub fn peek_is_or_eof(&mut self, token: impl Borrow<CTokenKind>) -> bool {
        self.peek_is(token) || self.peek().is_end_of_file()
    }

    pub fn advance(&mut self) -> &CToken {
        *self.stack.last_mut().unwrap() += 1;

        self.tokens
            .get(*self.stack.last().unwrap())
            .unwrap_or_else(|| self.eof())
    }

    pub fn speculate(&mut self) {
        self.stack.push(*self.stack.last().unwrap());
    }

    pub fn backtrack(&mut self) {
        self.stack.pop();
        assert!(!self.stack.is_empty());
    }

    pub fn success(&mut self) {
        // Indicates that a speculation was successful,
        // so we can remove the backtrack point

        let success = self.stack.pop().unwrap();
        let _ = self.stack.pop().unwrap();
        self.stack.push(success);
    }
}
