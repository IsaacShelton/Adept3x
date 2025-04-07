use inflow::Inflow;
use source_files::{Source, SourceFileKey, SourceFiles};
use std::{borrow::Borrow, fmt::Debug};
use token::{Token, TokenKind};

pub struct Input<'a, I: Inflow<Token>> {
    source_files: &'a SourceFiles,
    inflow: I,
    key: SourceFileKey,
}

impl<'a, I: Inflow<Token>> Debug for Input<'a, I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Input<Inflow<Token>>").finish()
    }
}

impl<'a, I> Input<'a, I>
where
    I: Inflow<Token>,
{
    pub fn new(inflow: I, source_files: &'a SourceFiles, key: SourceFileKey) -> Self {
        Self {
            inflow,
            source_files,
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

    pub fn eat_remember(&mut self, token: impl Borrow<TokenKind>) -> Option<Source> {
        if self.peek_is(token) {
            Some(self.advance().source)
        } else {
            None
        }
    }

    pub fn eat_identifier(&mut self) -> Option<String> {
        self.peek()
            .kind
            .is_identifier()
            .then(|| self.advance().kind.unwrap_identifier())
    }

    pub fn eat_polymorph(&mut self) -> Option<String> {
        self.peek()
            .kind
            .is_polymorph()
            .then(|| self.advance().kind.unwrap_polymorph())
    }

    pub fn ignore_newlines(&mut self) {
        while self.eat(TokenKind::Newline) {}
    }

    pub fn filename(&self) -> &str {
        self.source_files.get(self.key).filename()
    }

    pub fn key(&self) -> SourceFileKey {
        self.key
    }

    pub fn source_files(&self) -> &'a SourceFiles {
        self.source_files
    }

    // Adds input to the front of the queue,
    // useful for partially consuming tokens during parsing.
    pub fn unadvance(&mut self, token: Token) {
        self.inflow.un_next(token)
    }
}
