use crate::{
    line_column::Location,
    look_ahead::LookAhead,
    repeating_last::RepeatingLast,
    token::{Token, TokenInfo},
};
use std::borrow::Borrow;

pub struct Input<I: Iterator<Item = TokenInfo>> {
    iterator: LookAhead<RepeatingLast<I>>,
    filename: String,
}

impl<I> Input<I>
where
    I: Iterator<Item = TokenInfo>,
{
    pub fn new(iterator: I, filename: String) -> Self {
        Self {
            iterator: LookAhead::new(RepeatingLast::new(iterator)),
            filename,
        }
    }

    pub fn peek(&mut self) -> &TokenInfo {
        self.iterator.peek_nth(0).unwrap()
    }

    pub fn peek_nth(&mut self, n: usize) -> &TokenInfo {
        self.iterator.peek_nth(n).unwrap()
    }

    pub fn peek_is(&mut self, token: impl Borrow<Token>) -> bool {
        self.peek().token == *token.borrow()
    }

    pub fn peek_is_or_eof(&mut self, token: impl Borrow<Token>) -> bool {
        let next = &self.peek().token;
        next == token.borrow() || next.is_end_of_file()
    }

    pub fn advance(&mut self) -> TokenInfo {
        self.iterator.next().unwrap()
    }

    pub fn filename(&self) -> &str {
        &self.filename
    }
}
