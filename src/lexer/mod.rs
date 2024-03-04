mod is_character;
mod number_state;
mod state;
mod string_state;
mod hex_number_state;
mod identifier_state;

use crate::{
    line_column::LineColumn,
    look_ahead::LookAhead,
    token::{StringModifier, Token, TokenInfo},
};
use self::{
    string_state::StringState,
    number_state::NumberState,
    hex_number_state::HexNumberState,
    identifier_state::IdentifierState,
};
use is_character::IsCharacter;

use self::state::State;

pub struct Lexer<I: Iterator<Item = char>> {
    characters: LookAhead<LineColumn<I>>,
    state: State,
}

enum FeedResult<T> {
    Has(T),
    Waiting,
    Done,
}

impl<I> Lexer<I>
where
    I: Iterator<Item = char>,
{
    pub fn new(characters: I) -> Self {
        Self {
            characters: LookAhead::new(LineColumn::new(characters)),
            state: State::Idle,
        }
    }

    fn feed(&mut self) -> FeedResult<TokenInfo> {
        match self.state {
            State::Idle => self.feed_idle(),
            State::Identifier(_) => self.feed_identifier(),
            State::String(_) => self.feed_string(),
            State::Number(_) => self.feed_number(),
            State::HexNumber(_) => self.feed_hex_number(),
        }
    }

    fn feed_idle(&mut self) -> FeedResult<TokenInfo> {
        use FeedResult::*;

        // Skip spaces
        while let Some((' ', _)) = self.characters.peek() {
            self.characters.next();
        }

        if let Some((c, location)) = self.characters.next() {
            match c {
                '\n' => Has(TokenInfo::new(Token::Newline, location)),
                '{' => Has(TokenInfo::new(Token::OpenCurly, location)),
                '}' => Has(TokenInfo::new(Token::CloseCurly, location)),
                '(' => Has(TokenInfo::new(Token::OpenParen, location)),
                ')' => Has(TokenInfo::new(Token::CloseParen, location)),
                '[' => Has(TokenInfo::new(Token::OpenBracket, location)),
                ']' => Has(TokenInfo::new(Token::CloseBracket, location)),
                '/' if self.characters.peek().is_character('/') => {
                    // Comment

                    let start_location = location;

                    if self.characters.peek_nth(1).is_character('/') {
                        // Documentation Comment

                        // Skip over '///'
                        self.characters.next();
                        self.characters.next();
                        self.characters.next();

                        // Skip over leading spaces
                        while self.characters.peek().is_character(' ') {
                            self.characters.next();
                        }

                        let mut comment = String::new();

                        while let Some((c, location)) = self.characters.peek() {
                            if *c == '\n' {
                                return Has(TokenInfo::new(Token::DocComment(comment), *location));
                            } else {
                                comment.push(self.characters.next().unwrap().0);
                            }
                        }

                        Has(TokenInfo::new(Token::DocComment(comment), start_location))
                    } else {
                        // Regular line comment

                        while let Some((c, _)) = self.characters.next() {
                            if c == '\n' {
                                return Has(TokenInfo::new(Token::Newline, start_location));
                            }
                        }

                        Done
                    }
                }
                '0'..='9' => {
                    self.state = match self.characters.peek() {
                        Some(('x' | 'X', hex_location)) => {
                            // Eat 0x
                            let hex_location = *hex_location;
                            self.characters.next();

                            if let Some((c, _)) = self.characters.next() {
                                State::HexNumber(HexNumberState {
                                    value: String::from(c),
                                    start_location: location,
                                })
                            } else {
                                return Has(TokenInfo::new(
                                    Token::Error("Expected hex number after '0x'".into()),
                                    hex_location,
                                ));
                            }
                        }
                        _ => State::Number(NumberState::new(String::from(c), location)),
                    };

                    Waiting
                }
                '.' => {
                    if self.characters.peek_nth(0).is_character('.') && self.characters.peek_nth(1).is_character('.') {
                        self.characters.next();
                        self.characters.next();
                        Has(TokenInfo::new(Token::Ellipsis, location))
                    } else {
                        Has(TokenInfo::new(Token::Member, location))
                    }
                },
                '+' => Has(TokenInfo::new(Token::Add, location)),
                '-' => Has(TokenInfo::new(Token::Subtract, location)),
                '*' => Has(TokenInfo::new(Token::Multiply, location)),
                '/' => Has(TokenInfo::new(Token::Divide, location)),
                '%' => Has(TokenInfo::new(Token::Modulus, location)),
                '=' if self.characters.peek().is_character('=') => {
                    self.characters.next();
                    Has(TokenInfo::new(Token::Equals, location))
                }
                '!' if self.characters.peek().is_character('=') => {
                    self.characters.next();
                    Has(TokenInfo::new(Token::NotEquals, location))
                }
                '<' if self.characters.peek().is_character('=') => {
                    self.characters.next();
                    Has(TokenInfo::new(Token::LessThanEq, location))
                }
                '>' if self.characters.peek().is_character('=') => {
                    self.characters.next();
                    Has(TokenInfo::new(Token::GreaterThanEq, location))
                }
                '<' => Has(TokenInfo::new(Token::LessThan, location)),
                '>' => Has(TokenInfo::new(Token::GreaterThan, location)),
                '!' => Has(TokenInfo::new(Token::Not, location)),
                ',' => Has(TokenInfo::new(Token::Comma, location)),
                ':' => Has(TokenInfo::new(Token::Colon, location)),
                '#' => Has(TokenInfo::new(Token::Hash, location)),
                '\"' => {
                    self.state = State::String(StringState {
                        value: String::new(),
                        closing_char: c,
                        modifier: StringModifier::Normal,
                        start_location: location,
                    });
                    Waiting
                }
                'c' if self.characters.peek().is_character('\"') => {
                    // C-String
                    self.state = State::String(StringState {
                        value: String::new(),
                        closing_char: self.characters.next().unwrap().0,
                        modifier: StringModifier::NullTerminated,
                        start_location: location,
                    });
                    Waiting
                }
                _ if c.is_alphabetic() || c == '_' => {
                    self.state = State::Identifier(IdentifierState {
                        identifier: String::from(c),
                        start_location: location,
                    });
                    Waiting
                }
                _ => Has(TokenInfo::new(
                    Token::Error(format!("Unexpected character '{}'", c)),
                    location,
                )),
            }
        } else {
            Done
        }
    }

    fn feed_identifier(&mut self) -> FeedResult<TokenInfo> {
        use FeedResult::*;

        let state = self.state.as_mut_identifier();

        if let Some((c, _)) = self.characters.peek() {
            if c.is_alphabetic() || c.is_ascii_digit() || *c == '_' {
                state.identifier.push(self.characters.next().unwrap().0);
                Waiting
            } else {
                let token = state.to_token_info();
                self.state = State::Idle;
                Has(token)
            }
        } else {
            let token = state.to_token_info();
            self.state = State::Idle;
            Has(token)
        }
    }

    fn feed_string(&mut self) -> FeedResult<TokenInfo> {
        use FeedResult::*;

        let state = self.state.as_mut_string();

        if let Some((c, c_location)) = self.characters.next() {
            if c == state.closing_char {
                let value = std::mem::replace(&mut state.value, String::default());
                let modifier = state.modifier;
                let start_location = state.start_location;
                self.state = State::Idle;

                Has(TokenInfo::new(
                    Token::String { value, modifier },
                    start_location,
                ))
            } else if c == '\\' {
                if let Some((next_c, _)) = self.characters.next() {
                    match next_c {
                        'n' => state.value.push('\n'),
                        'r' => state.value.push('\r'),
                        't' => state.value.push('\t'),
                        _ => state.value.push(next_c),
                    }

                    Waiting
                } else {
                    Has(TokenInfo::new(
                        Token::Error("Expected character after string esacpe '\\'".into()),
                        c_location,
                    ))
                }
            } else {
                state.value.push(c);
                Waiting
            }
        } else {
            Has(TokenInfo::new(
                Token::Error("Unclosed string literal".into()),
                state.start_location,
            ))
        }
    }

    fn feed_number(&mut self) -> FeedResult<TokenInfo> {
        use FeedResult::*;

        let state = self.state.as_mut_number();

        if let Some((c, _)) = self.characters.peek() {
            let c = *c;

            if c.is_ascii_digit() {
                state.can_neg = false;
                self.characters.next();
                state.value.push(c);
                Waiting
            } else if c == '.' && state.can_dot {
                state.can_dot = false;
                self.characters.next();
                state.value.push(c);
                Waiting
            } else if (c == 'e' || c == 'E') && state.can_exp {
                state.can_exp = false;
                state.can_neg = true;
                state.can_dot = false;
                self.characters.next();
                state.value.push(c);
                Waiting
            } else if c == '-' && state.can_neg {
                state.can_neg = false;
                self.characters.next();
                state.value.push(c);
                Waiting
            } else {
                let token = state.to_token_info();
                self.state = State::Idle;
                Has(token)
            }
        } else {
            let token = state.to_token_info();
            self.state = State::Idle;
            Has(token)
        }
    }

    fn feed_hex_number(&mut self) -> FeedResult<TokenInfo> {
        use FeedResult::*;

        let state = self.state.as_mut_hex_number();

        if let Some((c, _)) = self.characters.peek() {
            let c = *c;

            if c.is_ascii_hexdigit() {
                self.characters.next();
                state.value.push(c);
                Waiting
            } else {
                let token = state.to_token_info();
                self.state = State::Idle;
                Has(token)
            }
        } else {
            let token = state.to_token_info();
            self.state = State::Idle;
            Has(token)
        }
    }
}

impl<I> Iterator for Lexer<I>
where
    I: Iterator<Item = char>,
{
    type Item = TokenInfo;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.feed() {
                FeedResult::Done => return None,
                FeedResult::Waiting => (),
                FeedResult::Has(token_info) => return Some(token_info),
            }
        }
    }
}
