mod feed_result;
mod infinite_iterator;

use crate::feed_result::FeedResult;
use token::{ALL_DIRECTIVES, ALL_PUNCT_SORTED, Directive, Punct, StringLiteral, Token, TokenKind};
use util_text::{Character, Lexable};

pub struct Lexer<L, S>
where
    L: Lexable<S> + Send,
    S: Copy,
{
    lexable: L,
    state: State<S>,
}

pub enum State<S: Copy> {
    Idle,
    UnaryCall,
    Identifier(String, S),
    UnknownDirective(String, S),
    String(StringState<S>),
    UnterminatedString(S),
    EndOfFile(S),
}

pub struct StringState<S: Copy> {
    literal: String,
    close_char: char,
    escaped: bool,
    source: S,
}

impl<L, S> Lexer<L, S>
where
    L: Lexable<S> + Send,
    S: Copy,
{
    pub fn new(lexable: L) -> Self {
        Self {
            lexable,
            state: State::Idle,
        }
    }

    fn feed(&mut self) -> FeedResult<Token<S>> {
        match &mut self.state {
            State::Idle => self.feed_idle(),
            State::UnaryCall => {
                self.state = State::Idle;
                let operator = "\'";
                if let Ok(source) = self.lexable.eat_remember(operator) {
                    FeedResult::Has(TokenKind::Punct(Punct::new(operator)).at(source))
                } else {
                    FeedResult::Waiting
                }
            }
            State::Identifier(name, source) => {
                if self.lexable.peek().is_identifier_continue() {
                    name.push(self.lexable.next().unwrap().0);
                    FeedResult::Waiting
                } else {
                    let name = std::mem::take(name);
                    let source = *source;
                    self.state = State::UnaryCall;
                    FeedResult::Has(TokenKind::Identifier(name).at(source))
                }
            }
            State::String(string_state) => match self.lexable.next() {
                Character::At(c, _) => {
                    string_state.literal.push(c);

                    if c == string_state.close_char && !string_state.escaped {
                        let literal = std::mem::take(&mut string_state.literal);
                        let source = string_state.source;
                        self.state = State::Idle;
                        FeedResult::Has(TokenKind::String(StringLiteral { literal }).at(source))
                    } else {
                        string_state.escaped = !string_state.escaped && c == '\\';
                        FeedResult::Waiting
                    }
                }
                Character::End(eof_source) => {
                    let literal = std::mem::take(&mut string_state.literal);
                    let source = string_state.source;
                    self.state = State::UnterminatedString(eof_source);
                    FeedResult::Has(TokenKind::String(StringLiteral { literal }).at(source))
                }
            },
            State::UnterminatedString(source) => {
                let source = *source;
                self.state = State::Idle;
                FeedResult::Has(TokenKind::MissingStringTermination.at(source))
            }
            State::UnknownDirective(directive, source) => {
                if self.lexable.peek().is_identifier_continue() {
                    directive.push(self.lexable.next().unwrap().0);
                    FeedResult::Waiting
                } else {
                    let directive = std::mem::take(directive);
                    let source = *source;
                    self.state = State::Idle;
                    FeedResult::Has(
                        TokenKind::Directive(Directive::Unknown(directive.into())).at(source),
                    )
                }
            }
            State::EndOfFile(source) => FeedResult::Has(TokenKind::EndOfFile.at(*source)),
        }
    }

    fn feed_idle(&mut self) -> FeedResult<Token<S>> {
        if let Some((spacing_atom, source)) = self.lexable.eat_column_spacing_atom() {
            return FeedResult::Has(TokenKind::ColumnSpacing(spacing_atom).at(source));
        }

        for punct_str in [")", "]", "}"] {
            if let Ok(source) = self.lexable.eat_remember(punct_str) {
                self.state = State::UnaryCall;
                return FeedResult::Has(TokenKind::Punct(Punct::new(punct_str)).at(source));
            }
        }

        if let Some((atom, source)) = self.lexable.eat_line_spacing_atom() {
            return FeedResult::Has(TokenKind::LineSpacing(atom).at(source));
        }

        if self.lexable.peek().is_identifier_start() {
            self.state = State::Identifier("".into(), self.lexable.peek().source());
            return FeedResult::Waiting;
        }

        for punct_str in ALL_PUNCT_SORTED.iter().copied() {
            if let Ok(source) = self.lexable.eat_remember(punct_str) {
                return FeedResult::Has(TokenKind::Punct(Punct::new(punct_str)).at(source));
            }
        }

        if let Ok(source) = self.lexable.eat_remember("@") {
            for possible in ALL_DIRECTIVES.iter().copied() {
                if self.lexable.eat(possible) {
                    return FeedResult::Has(
                        TokenKind::Directive(Directive::Standard(possible)).at(source),
                    );
                }
            }

            self.state = State::UnknownDirective("".into(), source);
            return FeedResult::Waiting;
        }

        match self.lexable.next() {
            Character::At(c @ ('"' | '\''), source) => {
                self.state = State::String(StringState {
                    literal: c.into(),
                    close_char: c,
                    escaped: false,
                    source,
                });
                FeedResult::Waiting
            }
            Character::At(c, source) => FeedResult::Has(TokenKind::Error(c).at(source)),
            Character::End(source) => {
                self.state = State::EndOfFile(source);
                FeedResult::Waiting
            }
        }
    }
}
