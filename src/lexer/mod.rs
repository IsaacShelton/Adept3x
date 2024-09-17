mod compound_identifier_state;
mod hex_number_state;
mod identifier_state;
mod number_state;
mod state;
mod string_state;

use self::{
    hex_number_state::HexNumberState, identifier_state::IdentifierState, number_state::NumberState,
    state::State, string_state::StringState,
};
use crate::{
    inflow::InflowStream,
    lexical_utils::FeedResult,
    source_files::Source,
    text::{Character, Text},
    token::{StringLiteral, StringModifier, Token, TokenKind},
};
use compound_identifier_state::CompoundIdentifierState;

pub struct Lexer<T: Text + Send> {
    characters: T,
    state: State,
}

impl<T: Text + Send> Lexer<T> {
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
            State::CompoundIdentifier(_) => self.feed_compound_identifier(),
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
            if let Ok(source) = self.characters.eat_remember('<') {
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

                return Has(token_kind.at(source));
            }
        }

        match self.characters.next() {
            Character::At(c, source) => self.feed_idle_char(c, source),
            Character::End(source) => Has(TokenKind::EndOfFile.at(source)),
        }
    }

    fn feed_idle_char(&mut self, c: char, source: Source) -> FeedResult<Token> {
        use FeedResult::*;

        match c {
            '\n' => Has(TokenKind::Newline.at(source)),
            '{' => Has(TokenKind::OpenCurly.at(source)),
            '}' => Has(TokenKind::CloseCurly.at(source)),
            '(' => Has(TokenKind::OpenParen.at(source)),
            ')' => Has(TokenKind::CloseParen.at(source)),
            '[' => Has(TokenKind::OpenBracket.at(source)),
            ']' => Has(TokenKind::CloseBracket.at(source)),
            '/' if self.characters.eat('*') => {
                // Multi-line comment

                let mut nesting = 0;

                loop {
                    if self.characters.eat("/*") {
                        nesting += 1;
                    }

                    if self.characters.eat("*/") {
                        if nesting == 0 {
                            break;
                        } else {
                            nesting -= 1;
                        }
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

                    Has(TokenKind::DocComment(comment).at(source))
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
                            return Has(TokenKind::Error("Expected hex number after '0x'".into())
                                .at(hex_source));
                        }
                    }
                    _ => State::Number(NumberState::new(String::from(c), source)),
                };

                Waiting
            }
            '.' => {
                if self.characters.eat("..") {
                    Has(TokenKind::Ellipsis.at(source))
                } else if self.characters.eat('.') {
                    Has(TokenKind::Extend.at(source))
                } else {
                    Has(TokenKind::Member.at(source))
                }
            }
            '+' => {
                if self.characters.eat('=') {
                    Has(TokenKind::AddAssign.at(source))
                } else {
                    Has(TokenKind::Add.at(source))
                }
            }
            '-' => {
                if self.characters.eat('=') {
                    Has(TokenKind::SubtractAssign.at(source))
                } else {
                    Has(TokenKind::Subtract.at(source))
                }
            }
            '*' => {
                if self.characters.eat('=') {
                    Has(TokenKind::MultiplyAssign.at(source))
                } else if self.characters.peek().is_spacing() {
                    Has(TokenKind::Multiply.at(source))
                } else {
                    Has(TokenKind::Dereference.at(source))
                }
            }
            '/' => {
                if self.characters.eat('=') {
                    Has(TokenKind::DivideAssign.at(source))
                } else {
                    Has(TokenKind::Divide.at(source))
                }
            }
            '%' => {
                if self.characters.eat('=') {
                    Has(TokenKind::ModulusAssign.at(source))
                } else {
                    Has(TokenKind::Modulus.at(source))
                }
            }
            '=' => {
                if self.characters.eat('=') {
                    Has(TokenKind::Equals.at(source))
                } else if self.characters.eat('>') {
                    Has(TokenKind::FatArrow.at(source))
                } else {
                    Has(TokenKind::Assign.at(source))
                }
            }
            '!' if self.characters.eat('=') => {
                self.characters.next();
                Has(TokenKind::NotEquals.at(source))
            }
            '>' if self.characters.eat('=') => {
                self.characters.next();
                Has(TokenKind::GreaterThanEq.at(source))
            }
            '>' if self.characters.eat(">>=") => Has(TokenKind::LogicalRightShiftAssign.at(source)),
            '>' if self.characters.eat(">>") => Has(TokenKind::LogicalRightShift.at(source)),
            '>' if self.characters.eat(">=") => Has(TokenKind::RightShiftAssign.at(source)),
            '>' if self.characters.eat('>') => Has(TokenKind::RightShift.at(source)),
            '>' => Has(TokenKind::GreaterThan.at(source)),
            '<' => Has(TokenKind::OpenAngle.at(source)),
            '!' => Has(TokenKind::Not.at(source)),
            '~' => Has(TokenKind::BitComplement.at(source)),
            '&' => {
                if self.characters.eat('=') {
                    Has(TokenKind::BitAndAssign.at(source))
                } else if self.characters.eat('&') {
                    Has(TokenKind::And.at(source))
                } else if self.characters.peek().is_spacing() {
                    Has(TokenKind::BitAnd.at(source))
                } else {
                    Has(TokenKind::AddressOf.at(source))
                }
            }
            '|' => {
                if self.characters.eat('=') {
                    Has(TokenKind::BitOrAssign.at(source))
                } else if self.characters.eat('|') {
                    Has(TokenKind::Or.at(source))
                } else {
                    Has(TokenKind::BitOr.at(source))
                }
            }
            '^' => {
                if self.characters.eat('=') {
                    Has(TokenKind::BitXorAssign.at(source))
                } else {
                    Has(TokenKind::BitXor.at(source))
                }
            }
            ',' => Has(TokenKind::Comma.at(source)),
            ':' if self.characters.eat('=') => Has(TokenKind::DeclareAssign.at(source)),
            ':' if self.characters.eat(':') => Has(TokenKind::Namespace.at(source)),
            ':' => Has(TokenKind::Colon.at(source)),
            '#' => Has(TokenKind::Hash.at(source)),
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
            _ => Has(TokenKind::Error(format!("Unexpected character '{}'", c)).at(source)),
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
            Character::At('/', _) if self.characters.peek_nth(1).is_alphabetic() => {
                state.identifier.push(self.characters.next().unwrap().0);
                Waiting
            }
            Character::At('<', _)
                if matches!(state.identifier.as_str(), "struct" | "union" | "enum") =>
            {
                let mut state = std::mem::replace(&mut self.state, State::Idle).unwrap_identifier();
                state.identifier.push('<');

                self.state = State::CompoundIdentifier(CompoundIdentifierState {
                    identifier: state.identifier,
                    start_source: state.start_source,
                });

                Waiting
            }
            _ => {
                let token = state.finalize();
                self.state = State::Idle;
                Has(token)
            }
        }
    }

    fn feed_compound_identifier(&mut self) -> FeedResult<Token> {
        let state = self.state.as_mut_compound_identifier();

        if let Some(token) = state.feed(self.characters.next()) {
            self.state = State::Idle;
            FeedResult::Has(token)
        } else {
            FeedResult::Waiting
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
                        Has(
                            TokenKind::Error("Expected character after string esacpe '\\'".into())
                                .at(c_source),
                        )
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

impl<T: Text + Send> InflowStream for Lexer<T> {
    type Item = Token;

    fn next(&mut self) -> Self::Item {
        loop {
            match self.feed() {
                FeedResult::Waiting => (),
                FeedResult::Has(token) => return token,
            }
        }
    }
}
