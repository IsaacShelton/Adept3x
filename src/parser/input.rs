use crate::{
    inflow::Inflow,
    source_file_cache::{SourceFileCache, SourceFileCacheKey},
    token::{Token, TokenKind},
};
use std::borrow::Borrow;

pub struct Input<'a, I: Inflow<Token>> {
    source_file_cache: &'a SourceFileCache,
    inflow: I,
    key: SourceFileCacheKey,
}

impl<'a, I> Input<'a, I>
where
    I: Inflow<Token>,
{
    pub fn new(inflow: I, source_file_cache: &'a SourceFileCache, key: SourceFileCacheKey) -> Self {
        Self {
            inflow,
            source_file_cache,
            key,
        }
    }

    pub fn peek(&mut self) -> &Token {
        self.inflow.peek_nth(0)
    }

    pub fn peek_nth(&mut self, n: usize) -> &Token {
        self.inflow.peek_nth(n)
    }

    pub fn peek_n<const N: usize>(&mut self) -> [&Token; N] {
        self.inflow.peek_n::<N>()
    }

    pub fn peek_is(&mut self, token: impl Borrow<TokenKind>) -> bool {
        self.peek().kind == *token.borrow()
    }

    pub fn peek_is_or_eof(&mut self, token: impl Borrow<TokenKind>) -> bool {
        let next = &self.peek().kind;
        next == token.borrow() || next.is_end_of_file()
    }

    pub fn advance(&mut self) -> Token {
        self.inflow.next()
    }

    pub fn eat(&mut self, token: impl Borrow<TokenKind>) -> bool {
        if self.peek_is(token) {
            self.advance();
            true
        } else {
            false
        }
    }

    pub fn eat_identifier(&mut self) -> Option<String> {
        self.peek()
            .kind
            .is_identifier()
            .then(|| self.advance().kind.unwrap_identifier())
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

    // Adds input to the front of the queue,
    // useful for partially consuming tokens during parsing.
    pub fn unadvance(&mut self, token: Token) {
        self.inflow.un_next(token)
    }
}
