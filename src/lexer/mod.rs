mod hex_number_state;
mod identifier_state;
mod number_state;
mod state;
mod string_state;

use self::state::State;
use self::{
    hex_number_state::HexNumberState, identifier_state::IdentifierState, number_state::NumberState,
    string_state::StringState,
};
use crate::ast::Source;
use crate::inflow::InflowStream;
use crate::lexical_utils::FeedResult;
use crate::text::Character;
use crate::text::Text;
use crate::token::{StringLiteral, StringModifier, Token, TokenKind};

/*
    NOTE: We guarantee that lexer returns `Some(Token { kind : TokenKind::EndOfFile, .. })`
    before returning `None`. Prehaps it should be migrated to use `InflowStream` instead?
*/
pub struct Lexer<T: Text> {
    characters: T,
    state: State,
}

impl<T: Text> Lexer<T> {
    pub fn new(characters: T) -> Self {
        Self {
            characters,
            state: State::Idle,
        }
    }

    fn feed(&mut self) -> FeedResult<Token> {
        match &self.state {
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
        while self.characters.peek().is(' ') {
            self.characters.next();

            // Special case for operators '<', '<=', etc.
            // These require a preceeding space right now, but might be made more permissive later.
            // (at the very least '<' needs a preceeding space to disambiguate between less-than and left-angle)
            // (this seems to be the best syntax trade off as everybody uses a preceeding space anyway)
            if let Character::At('<', source) = self.characters.peek() {
                self.characters.next();

                // TODO: CLEANUP: This could be better
                let (token_kind, num_extra_chars) =
                    match &self.characters.peek_n::<3>().map(Character::or_nul) {
                        ['<', '<', '=', ..] => (TokenKind::LogicalLeftShiftAssign, 3),
                        ['<', '<', ..] => (TokenKind::LogicalLeftShift, 2),
                        ['<', '=', ..] => (TokenKind::LeftShiftAssign, 2),
                        ['<', ..] => (TokenKind::LeftShift, 1),
                        ['=', ..] => (TokenKind::LessThanEq, 1),
                        _ => (TokenKind::LessThan, 0),
                    };

                for _ in 0..num_extra_chars {
                    self.characters.next();
                }

                return Has(Token::new(token_kind, source));
            }
        }

        match self.characters.next() {
            Character::At(c, source) => self.feed_idle_char(c, source),
            Character::End(source) => Has(Token::new(TokenKind::EndOfFile, source)),
        }
    }

    fn feed_idle_char(&mut self, c: char, source: Source) -> FeedResult<Token> {
        use FeedResult::*;

        match c {
            '\n' => Has(Token::new(TokenKind::Newline, source)),
            '{' => Has(Token::new(TokenKind::OpenCurly, source)),
            '}' => Has(Token::new(TokenKind::CloseCurly, source)),
            '(' => Has(Token::new(TokenKind::OpenParen, source)),
            ')' => Has(Token::new(TokenKind::CloseParen, source)),
            '[' => Has(Token::new(TokenKind::OpenBracket, source)),
            ']' => Has(Token::new(TokenKind::CloseBracket, source)),
            '/' if self.characters.eat('*') => {
                // Multi-line comment

                loop {
                    if self.characters.eat("*/") {
                        break;
                    }

                    if self.characters.peek().is_end() {
                        return Has(TokenKind::Error("Unterminated line comment".into()).at(source));
                    }

                    self.characters.next();
                }

                Waiting
            }
            '/' if self.characters.eat('/') => {
                // Comment

                if self.characters.eat('/') {
                    // Documentation Comment

                    // Skip over leading spaces
                    while self.characters.eat(' ') {}

                    let mut comment = String::new();

                    while let Character::At(c, _) = self.characters.next() {
                        match c {
                            '\n' => break,
                            _ => comment.push(c),
                        }
                    }

                    Has(Token::new(TokenKind::DocComment(comment), source))
                } else {
                    // Regular line comment

                    while let Character::At(c, _) = self.characters.next() {
                        match c {
                            '\n' => break,
                            _ => (),
                        }
                    }

                    Waiting
                }
            }
            '0'..='9' => {
                self.state = match self.characters.peek() {
                    Character::At('x' | 'X', hex_source) => {
                        // Eat x of 0x
                        self.characters.next();

                        if let Character::At(c, _) = self.characters.next() {
                            State::HexNumber(HexNumberState {
                                value: String::from(c),
                                start_source: source,
                            })
                        } else {
                            return Has(Token::new(
                                TokenKind::Error("Expected hex number after '0x'".into()),
                                hex_source,
                            ));
                        }
                    }
                    _ => State::Number(NumberState::new(String::from(c), source)),
                };

                Waiting
            }
            '.' => {
                if self.characters.eat("..") {
                    Has(Token::new(TokenKind::Ellipsis, source))
                } else if self.characters.eat('.') {
                    Has(Token::new(TokenKind::Extend, source))
                } else {
                    Has(Token::new(TokenKind::Member, source))
                }
            }
            '+' => {
                if self.characters.eat('=') {
                    Has(Token::new(TokenKind::AddAssign, source))
                } else {
                    Has(Token::new(TokenKind::Add, source))
                }
            }
            '-' => {
                if self.characters.eat('=') {
                    Has(Token::new(TokenKind::SubtractAssign, source))
                } else {
                    Has(Token::new(TokenKind::Subtract, source))
                }
            }
            '*' => {
                if self.characters.eat('=') {
                    Has(Token::new(TokenKind::MultiplyAssign, source))
                } else {
                    Has(Token::new(TokenKind::Multiply, source))
                }
            }
            '/' => {
                if self.characters.eat('=') {
                    Has(Token::new(TokenKind::DivideAssign, source))
                } else {
                    Has(Token::new(TokenKind::Divide, source))
                }
            }
            '%' => {
                if self.characters.eat('=') {
                    Has(Token::new(TokenKind::ModulusAssign, source))
                } else {
                    Has(Token::new(TokenKind::Modulus, source))
                }
            }
            '=' => {
                if self.characters.eat('=') {
                    Has(Token::new(TokenKind::Equals, source))
                } else {
                    Has(Token::new(TokenKind::Assign, source))
                }
            }
            '!' if self.characters.eat('=') => {
                self.characters.next();
                Has(Token::new(TokenKind::NotEquals, source))
            }
            '>' if self.characters.eat('=') => {
                self.characters.next();
                Has(Token::new(TokenKind::GreaterThanEq, source))
            }
            '>' if self.characters.eat(">>=") => {
                Has(Token::new(TokenKind::LogicalRightShiftAssign, source))
            }
            '>' if self.characters.eat(">>") => {
                Has(Token::new(TokenKind::LogicalRightShift, source))
            }
            '>' if self.characters.eat(">=") => {
                Has(Token::new(TokenKind::RightShiftAssign, source))
            }
            '>' if self.characters.eat('>') => Has(Token::new(TokenKind::RightShift, source)),
            '>' => Has(Token::new(TokenKind::GreaterThan, source)),
            '<' => Has(Token::new(TokenKind::OpenAngle, source)),
            '!' => Has(Token::new(TokenKind::Not, source)),
            '~' => Has(Token::new(TokenKind::BitComplement, source)),
            '&' => {
                if self.characters.eat('=') {
                    Has(Token::new(TokenKind::AmpersandAssign, source))
                } else if self.characters.eat('&') {
                    Has(Token::new(TokenKind::And, source))
                } else {
                    Has(Token::new(TokenKind::Ampersand, source))
                }
            }
            '|' => {
                if self.characters.eat('=') {
                    Has(Token::new(TokenKind::PipeAssign, source))
                } else if self.characters.eat('|') {
                    Has(Token::new(TokenKind::Or, source))
                } else {
                    Has(Token::new(TokenKind::Pipe, source))
                }
            }
            '^' => {
                if self.characters.eat('=') {
                    Has(Token::new(TokenKind::CaretAssign, source))
                } else {
                    Has(Token::new(TokenKind::Caret, source))
                }
            }
            ',' => Has(Token::new(TokenKind::Comma, source)),
            ':' if self.characters.eat('=') => Has(Token::new(TokenKind::DeclareAssign, source)),
            ':' if self.characters.eat(':') => Has(Token::new(TokenKind::Namespace, source)),
            ':' => Has(Token::new(TokenKind::Colon, source)),
            '#' => Has(Token::new(TokenKind::Hash, source)),
            '\"' => {
                self.state = State::String(StringState {
                    value: String::new(),
                    closing_char: c,
                    modifier: StringModifier::Normal,
                    start_source: source,
                });
                Waiting
            }
            'c' if self.characters.peek().is('\"') => {
                // C-String
                self.state = State::String(StringState {
                    value: String::new(),
                    closing_char: self.characters.next().unwrap().0,
                    modifier: StringModifier::NullTerminated,
                    start_source: source,
                });
                Waiting
            }
            _ if c.is_alphabetic() || c == '_' => {
                self.state = State::Identifier(IdentifierState {
                    identifier: String::from(c),
                    start_source: source,
                });
                Waiting
            }
            _ => Has(Token::new(
                TokenKind::Error(format!("Unexpected character '{}'", c)),
                source,
            )),
        }
    }

    fn feed_identifier(&mut self) -> FeedResult<Token> {
        use FeedResult::*;

        let state = self.state.as_mut_identifier();

        match self.characters.peek() {
            Character::At(c, _) if c.is_alphabetic() || c.is_ascii_digit() || c == '_' => {
                state.identifier.push(self.characters.next().unwrap().0);
                Waiting
            }
            _ => {
                let token = state.finalize();
                self.state = State::Idle;
                Has(token)
            }
        }
    }

    fn feed_string(&mut self) -> FeedResult<Token> {
        use FeedResult::*;

        let state = self.state.as_mut_string();

        match self.characters.next() {
            Character::At(c, c_source) => {
                if c == state.closing_char {
                    let value = std::mem::take(&mut state.value);
                    let modifier = state.modifier;
                    let start_source = state.start_source;
                    self.state = State::Idle;

                    Has(TokenKind::String(StringLiteral { value, modifier }).at(start_source))
                } else if c == '\\' {
                    if let Character::At(next_c, _) = self.characters.next() {
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
                            c_source,
                        ))
                    }
                } else {
                    state.value.push(c);
                    Waiting
                }
            }
            Character::End(_) => {
                Has(TokenKind::Error("Unclosed string literal".into()).at(state.start_source))
            }
        }
    }

    fn feed_number(&mut self) -> FeedResult<Token> {
        use FeedResult::*;

        let state = self.state.as_mut_number();

        if self.characters.peek().is_digit() {
            state.can_neg = false;
            state.value.push(self.characters.next().unwrap().0);
            Waiting
        } else if state.can_dot && self.characters.eat('.') {
            state.can_dot = false;
            state.value.push('.');
            Waiting
        } else if state.can_exp && (self.characters.eat('e') || self.characters.eat('E')) {
            state.can_exp = false;
            state.can_neg = true;
            state.can_dot = false;
            state.value.push('e');
            Waiting
        } else if state.can_neg && self.characters.eat('-') {
            state.can_neg = false;
            state.value.push('-');
            Waiting
        } else {
            let token = state.to_token();
            self.state = State::Idle;
            Has(token)
        }
    }

    fn feed_hex_number(&mut self) -> FeedResult<Token> {
        use FeedResult::*;

        let state = self.state.as_mut_hex_number();

        if let Character::At(c, _) = self.characters.peek() {
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

impl<T: Text> InflowStream for Lexer<T> {
    type Item = Token;

    fn next(&mut self) -> Self::Item {
        loop {
            match self.feed() {
                FeedResult::Eof(eof) => return eof,
                FeedResult::Waiting => (),
                FeedResult::Has(token) => return token,
            }
        }
    }
}
