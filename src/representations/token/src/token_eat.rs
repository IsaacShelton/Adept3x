use crate::{StringLiteral, Token, TokenKind};
use infinite_iterator::InfinitePeekable;
use num_bigint::BigInt;

pub trait TokenEat<S> {
    fn eat(&mut self, kind: TokenKind) -> bool;
    fn eat_identifier(&mut self) -> Option<String>;
    fn eat_string(&mut self) -> Option<StringLiteral>;
    fn eat_integer(&mut self) -> Option<BigInt>;
    fn eat_newlines(&mut self);
}

impl<S, I: InfinitePeekable<Token<S>>> TokenEat<S> for I {
    fn eat(&mut self, kind: TokenKind) -> bool {
        (self.peek().kind == kind)
            .then(|| {
                self.next();
                true
            })
            .unwrap_or(false)
    }

    fn eat_identifier(&mut self) -> Option<String> {
        self.peek()
            .is_identifier()
            .then(|| self.next().kind.unwrap_identifier())
    }

    fn eat_string(&mut self) -> Option<StringLiteral> {
        self.peek()
            .is_string()
            .then(|| self.next().kind.unwrap_string())
    }

    fn eat_integer(&mut self) -> Option<BigInt> {
        self.peek()
            .is_integer()
            .then(|| self.next().kind.unwrap_integer())
    }

    fn eat_newlines(&mut self) {
        while self.peek().is_newline() {
            self.next();
        }
    }
}
