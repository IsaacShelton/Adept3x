mod hex_number_state;
mod identifier_state;
mod is_character;
mod number_state;
mod state;
mod string_state;

use self::{
    hex_number_state::HexNumberState, identifier_state::IdentifierState, number_state::NumberState,
    string_state::StringState,
};
use crate::{
    line_column::LineColumn,
    look_ahead::LookAhead,
    token::{StringLiteral, StringModifier, Token, TokenKind},
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

    fn feed(&mut self) -> FeedResult<Token> {
        match self.state {
            State::EndOfFile => FeedResult::Done,
            State::Idle => self.feed_idle(),
            State::Identifier(_) => self.feed_identifier(),
            State::String(_) => self.feed_string(),
            State::Number(_) => self.feed_number(),
            State::HexNumber(_) => self.feed_hex_number(),
        }
    }

    fn feed_idle(&mut self) -> FeedResult<Token> {
        use FeedResult::*;

        // Skip spaces
        while let Some((' ', _)) = self.characters.peek() {
            self.characters.next();

            // Special case for comparison operators '<' and '<='
            if let Some(('<', location)) = self.characters.peek() {
                let location = *location;
                self.characters.next();

                if let Some(('=', _)) = self.characters.peek() {
                    self.characters.next();
                    return Has(Token::new(TokenKind::LessThanEq, location));
                } else if let Some(('<', _)) = self.characters.peek() {
                    self.characters.next();

                    if let Some(('<', _)) = self.characters.peek() {
                        self.characters.next();
                        return Has(Token::new(TokenKind::LogicalLeftShift, location));
                    } else {
                        return Has(Token::new(TokenKind::LeftShift, location));
                    }
                } else {
                    return Has(Token::new(TokenKind::LessThan, location));
                }
            }
        }

        if let Some((c, location)) = self.characters.next() {
            match c {
                '\n' => Has(Token::new(TokenKind::Newline, location)),
                '{' => Has(Token::new(TokenKind::OpenCurly, location)),
                '}' => Has(Token::new(TokenKind::CloseCurly, location)),
                '(' => Has(Token::new(TokenKind::OpenParen, location)),
                ')' => Has(Token::new(TokenKind::CloseParen, location)),
                '[' => Has(Token::new(TokenKind::OpenBracket, location)),
                ']' => Has(Token::new(TokenKind::CloseBracket, location)),
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
                                return Has(Token::new(TokenKind::DocComment(comment), *location));
                            } else {
                                comment.push(self.characters.next().unwrap().0);
                            }
                        }

                        Has(Token::new(TokenKind::DocComment(comment), start_location))
                    } else {
                        // Regular line comment

                        while let Some((c, _)) = self.characters.next() {
                            if c == '\n' {
                                return Has(Token::new(TokenKind::Newline, start_location));
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
                                return Has(Token::new(
                                    TokenKind::Error("Expected hex number after '0x'".into()),
                                    hex_location,
                                ));
                            }
                        }
                        _ => State::Number(NumberState::new(String::from(c), location)),
                    };

                    Waiting
                }
                '.' => {
                    if self.characters.peek_nth(0).is_character('.')
                        && self.characters.peek_nth(1).is_character('.')
                    {
                        self.characters.next();
                        self.characters.next();
                        Has(Token::new(TokenKind::Ellipsis, location))
                    } else {
                        Has(Token::new(TokenKind::Member, location))
                    }
                }
                '+' => Has(Token::new(TokenKind::Add, location)),
                '-' => Has(Token::new(TokenKind::Subtract, location)),
                '*' => Has(Token::new(TokenKind::Multiply, location)),
                '/' => Has(Token::new(TokenKind::Divide, location)),
                '%' => Has(Token::new(TokenKind::Modulus, location)),
                '=' if self.characters.peek().is_character('=') => {
                    self.characters.next();
                    Has(Token::new(TokenKind::Equals, location))
                }
                '=' => Has(Token::new(TokenKind::Assign, location)),
                '!' if self.characters.peek().is_character('=') => {
                    self.characters.next();
                    Has(Token::new(TokenKind::NotEquals, location))
                }
                '>' if self.characters.peek().is_character('=') => {
                    self.characters.next();
                    Has(Token::new(TokenKind::GreaterThanEq, location))
                }
                '>' if self.characters.peek().is_character('>') && self.characters.peek_nth(1).is_character('>') => {
                    self.characters.next();
                    self.characters.next();
                    Has(Token::new(TokenKind::LogicalRightShift, location))
                }
                '>' if self.characters.peek().is_character('>') => {
                    self.characters.next();
                    Has(Token::new(TokenKind::RightShift, location))
                }
                '>' => Has(Token::new(TokenKind::GreaterThan, location)),
                '<' => Has(Token::new(TokenKind::OpenAngle, location)),
                '!' => Has(Token::new(TokenKind::Not, location)),
                '&' => Has(Token::new(TokenKind::Ampersand, location)),
                '|' => Has(Token::new(TokenKind::Pipe, location)),
                '^' => Has(Token::new(TokenKind::Caret, location)),
                ',' => Has(Token::new(TokenKind::Comma, location)),
                ':' if self.characters.peek().is_character('=') => {
                    self.characters.next();
                    Has(Token::new(TokenKind::DeclareAssign, location))
                }
                ':' => Has(Token::new(TokenKind::Colon, location)),
                '#' => Has(Token::new(TokenKind::Hash, location)),
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
                _ => Has(Token::new(
                    TokenKind::Error(format!("Unexpected character '{}'", c)),
                    location,
                )),
            }
        } else {
            self.state = State::EndOfFile;

            Has(Token::new(
                TokenKind::EndOfFile,
                self.characters.friendly_location(),
            ))
        }
    }

    fn feed_identifier(&mut self) -> FeedResult<Token> {
        use FeedResult::*;

        let state = self.state.as_mut_identifier();

        if let Some((c, _)) = self.characters.peek() {
            if c.is_alphabetic() || c.is_ascii_digit() || *c == '_' {
                state.identifier.push(self.characters.next().unwrap().0);
                Waiting
            } else {
                let token = state.to_token();
                self.state = State::Idle;
                Has(token)
            }
        } else {
            let token = state.to_token();
            self.state = State::Idle;
            Has(token)
        }
    }

    fn feed_string(&mut self) -> FeedResult<Token> {
        use FeedResult::*;

        let state = self.state.as_mut_string();

        if let Some((c, c_location)) = self.characters.next() {
            if c == state.closing_char {
                let value = std::mem::replace(&mut state.value, String::default());
                let modifier = state.modifier;
                let start_location = state.start_location;
                self.state = State::Idle;

                Has(Token::new(
                    TokenKind::String(StringLiteral { value, modifier }),
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
                    Has(Token::new(
                        TokenKind::Error("Expected character after string esacpe '\\'".into()),
                        c_location,
                    ))
                }
            } else {
                state.value.push(c);
                Waiting
            }
        } else {
            Has(Token::new(
                TokenKind::Error("Unclosed string literal".into()),
                state.start_location,
            ))
        }
    }

    fn feed_number(&mut self) -> FeedResult<Token> {
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
                let token = state.to_token();
                self.state = State::Idle;
                Has(token)
            }
        } else {
            let token = state.to_token();
            self.state = State::Idle;
            Has(token)
        }
    }

    fn feed_hex_number(&mut self) -> FeedResult<Token> {
        use FeedResult::*;

        let state = self.state.as_mut_hex_number();

        if let Some((c, _)) = self.characters.peek() {
            let c = *c;

            if c.is_ascii_hexdigit() {
                self.characters.next();
                state.value.push(c);
                Waiting
            } else {
                let token = state.to_token();
                self.state = State::Idle;
                Has(token)
            }
        } else {
            let token = state.to_token();
            self.state = State::Idle;
            Has(token)
        }
    }
}

impl<I> Iterator for Lexer<I>
where
    I: Iterator<Item = char>,
{
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.feed() {
                FeedResult::Done => return None,
                FeedResult::Waiting => (),
                FeedResult::Has(token) => return Some(token),
            }
        }
    }
}
