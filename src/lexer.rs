
use crate::look_ahead::LookAhead;

pub struct Lexer<I: Iterator<Item = char>> {
    characters: LookAhead<I>,
    state: State,
}

enum FeedResult<T> {
    Has(T),
    Waiting,
    Done,
    Error(String),
}

impl<I> Lexer<I> where I: Iterator<Item = char> {
    pub fn new(characters: I) -> Self {
        Self {
            characters: LookAhead::new(characters),
            state: State::Idle,
        }
    }

    fn feed(&mut self) -> FeedResult<Token> {
        match self.state {
            State::Idle => self.feed_idle(),
            State::Identifier(_) => self.feed_identifier(),
        }
    }

    fn feed_idle(&mut self) -> FeedResult<Token> {
        // Ignore spaces
        while let Some(' ') = self.characters.peek() {
            self.characters.next();
        }
        
        if let Some(c) = self.characters.next() {
            if c == '\n' {
                FeedResult::Has(Token::Newline)
            } else if c.is_alphabetic() {
                self.state = State::Identifier(String::from(c));
                FeedResult::Waiting
            } else {
                FeedResult::Error(format!("Unexpected character {}", c))
            }
        } else {
            FeedResult::Done
        }
    }

    fn feed_identifier(&mut self) -> FeedResult<Token> {
        let state = self.state.as_mut_identifier();

        if let Some(c) = self.characters.peek() {
            if c.is_alphabetic() || c.is_ascii_digit() || *c == '_' {
                state.push(self.characters.next().unwrap());
                FeedResult::Waiting
            } else {
                let identifier = std::mem::replace(state, String::default());
                self.state = State::Idle;
                FeedResult::Has(Token::Identifier(identifier))
            }
        } else {
            let identifier = std::mem::replace(state, String::default());
            self.state = State::Idle;
            FeedResult::Has(Token::Identifier(identifier))
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    Error(String),
    Newline,
    Identifier(String),
}

impl<I> Iterator for Lexer<I> where I: Iterator<Item = char> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.feed() {
                FeedResult::Done => return None,
                FeedResult::Waiting => (),
                FeedResult::Has(token) => return Some(token),
                FeedResult::Error(error) => return Some(Token::Error(error)),
            }
        }
    }
}

enum State {
    Idle,
    Identifier(String),
}

impl State {
    pub fn as_mut_identifier(&mut self) -> &mut String {
        match self {
            State::Identifier(identifier) => identifier,
            _ => panic!(),
        }
    }
}

