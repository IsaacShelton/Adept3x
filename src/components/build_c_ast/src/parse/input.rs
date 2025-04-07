use c_token::{CToken, CTokenKind};
use source_files::{Source, SourceFileKey, SourceFiles};
use std::borrow::Borrow;

pub struct Input<'a> {
    source_files: &'a SourceFiles,
    key: SourceFileKey,
    tokens: Vec<CToken>,
    stack: Vec<usize>,
}

impl<'a> Input<'a> {
    pub fn new(tokens: Vec<CToken>, source_files: &'a SourceFiles, key: SourceFileKey) -> Self {
        assert!(tokens.last().map_or(false, |token| token.is_end_of_file()));

        Self {
            source_files,
            tokens,
            key,
            stack: vec![0],
        }
    }

    pub fn switch_input(&mut self, mut tokens: Vec<CToken>) {
        if !tokens.last().map_or(false, |token| token.is_end_of_file()) {
            tokens.push(CTokenKind::EndOfFile.at(Source::internal()));
        }

        self.tokens = tokens;

        // Reset stack
        self.stack.clear();
        self.stack.push(0);
    }

    pub fn filename(&self) -> &str {
        self.source_files.get(self.key).filename()
    }

    pub fn source_files(&self) -> &'a SourceFiles {
        self.source_files
    }

    pub fn key(&self) -> SourceFileKey {
        self.key
    }

    pub fn eof(&self) -> &CToken {
        self.tokens.last().unwrap()
    }

    pub fn here(&self) -> Source {
        self.peek().source
    }

    pub fn peek(&self) -> &CToken {
        self.tokens
            .get(*self.stack.last().unwrap())
            .unwrap_or_else(|| self.eof())
    }

    pub fn peek_nth(&self, n: usize) -> &CToken {
        self.tokens
            .get(self.stack.last().unwrap() + n)
            .unwrap_or_else(|| self.eof())
    }

    pub fn peek_n(&self, n: usize) -> &[CToken] {
        let start = *self.stack.last().unwrap();
        let end = (start + n).min(self.tokens.len());
        &self.tokens[start..end]
    }

    pub fn peek_is(&mut self, token: impl Borrow<CTokenKind>) -> bool {
        self.peek().kind == *token.borrow()
    }

    pub fn peek_is_or_eof(&mut self, token: impl Borrow<CTokenKind>) -> bool {
        self.peek_is(token) || self.peek().is_end_of_file()
    }

    pub fn advance(&mut self) -> &CToken {
        let index = *self.stack.last().unwrap();

        if let Some(token) = self.tokens.get(index) {
            *self.stack.last_mut().unwrap() += 1;
            token
        } else {
            self.eof()
        }
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
