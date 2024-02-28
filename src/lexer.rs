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

impl<I> Lexer<I>
where
    I: Iterator<Item = char>,
{
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
            State::String { .. } => self.feed_string(),
        }
    }

    fn feed_idle(&mut self) -> FeedResult<Token> {
        // Skip spaces
        while let Some(' ') = self.characters.peek() {
            self.characters.next();
        }

        if let Some(c) = self.characters.next() {
            match c {
                '\n' => FeedResult::Has(Token::Newline),
                '{' => FeedResult::Has(Token::OpenCurly),
                '}' => FeedResult::Has(Token::CloseCurly),
                '(' => FeedResult::Has(Token::OpenParen),
                ')' => FeedResult::Has(Token::CloseParen),
                '[' => FeedResult::Has(Token::OpenBracket),
                ']' => FeedResult::Has(Token::CloseBracket),
                '/' if self.characters.peek() == Some(&'/') => {
                    // Comment

                    if let Some(&'/') = self.characters.peek_nth(1) {
                        // Documentation Comment

                        // Skip over '///'
                        self.characters.next();
                        self.characters.next();
                        self.characters.next();

                        // Skip over leading spaces
                        while let Some(&' ') = self.characters.peek() {
                            self.characters.next();
                        }

                        let mut comment = String::new();

                        while let Some(c) = self.characters.peek() {
                            if *c == '\n' {
                                return FeedResult::Has(Token::DocComment(comment));
                            } else {
                                comment.push(self.characters.next().unwrap());
                            }
                        }

                        FeedResult::Has(Token::DocComment(comment))
                    } else {
                        // Regular line comment

                        while let Some(c) = self.characters.next() {
                            if c == '\n' {
                                return FeedResult::Has(Token::Newline);
                            }
                        }

                        FeedResult::Done
                    }
                }
                '\"' => {
                    self.state = State::String(StringState {
                        value: String::new(),
                        closing_char: c,
                        modifier: StringModifier::Normal,
                    });
                    FeedResult::Waiting
                }
                'c' if self.characters.peek() == Some(&'\"') => {
                    // C-String
                    self.state = State::String(StringState {
                        value: String::new(),
                        closing_char: self.characters.next().unwrap(),
                        modifier: StringModifier::NullTerminated,
                    });
                    FeedResult::Waiting
                }
                _ if c.is_alphabetic() => {
                    self.state = State::Identifier(String::from(c));
                    FeedResult::Waiting
                }
                _ => FeedResult::Error(format!("Unexpected character {}", c)),
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

    fn feed_string(&mut self) -> FeedResult<Token> {
        let state = self.state.as_mut_string();

        if let Some(c) = self.characters.next() {
            if c == state.closing_char {
                let value = std::mem::replace(&mut state.value, String::default());
                let modifier = state.modifier;
                self.state = State::Idle;
                FeedResult::Has(Token::String { value, modifier })
            } else if c == '\\' {
                if let Some(next_c) = self.characters.next() {
                    match next_c {
                        'n' => state.value.push('\n'),
                        'r' => state.value.push('\r'),
                        't' => state.value.push('\t'),
                        _ => state.value.push(next_c),
                    }

                    FeedResult::Waiting
                } else {
                    FeedResult::Error("Expected character after string esacpe '\\'".into())
                }
            } else {
                state.value.push(c);
                FeedResult::Waiting
            }
        } else {
            FeedResult::Error("Unclosed string literal".into())
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    Error(String),
    Newline,
    Identifier(String),
    OpenCurly,
    CloseCurly,
    OpenParen,
    CloseParen,
    OpenBracket,
    CloseBracket,
    String {
        value: String,
        modifier: StringModifier,
    },
    DocComment(String),
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
                FeedResult::Error(error) => return Some(Token::Error(error)),
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum StringModifier {
    Normal,
    NullTerminated,
}

struct StringState {
    value: String,
    closing_char: char,
    modifier: StringModifier,
}

enum State {
    Idle,
    Identifier(String),
    String(StringState),
}

impl State {
    pub fn as_mut_identifier(&mut self) -> &mut String {
        match self {
            State::Identifier(identifier) => identifier,
            _ => panic!(),
        }
    }

    pub fn as_mut_string(&mut self) -> &mut StringState {
        match self {
            State::String(state) => state,
            _ => panic!(),
        }
    }
}
