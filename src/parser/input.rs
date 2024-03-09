use crate::{line_column::Location, look_ahead::LookAhead, token::{Token, TokenInfo}};

pub struct Input<I: Iterator<Item = TokenInfo>> {
    iterator: LookAhead<I>,
    previous_location: Location,
    filename: String,
}

impl<I> Input<I>
where
    I: Iterator<Item = TokenInfo>,
{
    pub fn new(iterator: I, filename: String) -> Self {
        Self {
            iterator: LookAhead::new(iterator),
            previous_location: Location::new(1, 1),
            filename,
        }
    }

    pub fn peek(&mut self) -> Option<&TokenInfo> {
        self.iterator.peek()
    }

    pub fn peek_nth(&mut self, n: usize) -> Option<&TokenInfo> {
        self.iterator.peek_nth(n)
    }

    pub fn peek_is(&mut self, token: Token) -> bool {
        if let Some(token_info) = self.iterator.peek() {
            token_info.token == token
        } else {
            false
        }
    }

    pub fn peek_is_or_eof(&mut self, token: Token) -> bool {
        if let Some(token_info) = self.iterator.peek() {
            token_info.token == token
        } else {
            true
        }
    }

    pub fn next(&mut self) -> Option<TokenInfo> {
        self.iterator.next().map(|token_info| {
            self.previous_location = token_info.location;
            token_info
        })
    }

    pub fn previous_location(&self) -> Location {
        self.previous_location
    }

    pub fn filename(&self) -> &str {
        &self.filename
    }
}

