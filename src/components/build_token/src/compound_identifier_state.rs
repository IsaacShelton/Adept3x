use text::Character;
use token::{Token, TokenKind};

pub struct CompoundIdentifierState<S: Copy> {
    pub identifier: String,
    pub start_source: S,
}

impl<S: Copy> CompoundIdentifierState<S> {
    pub fn feed(&mut self, character: Character<S>) -> Option<Token<S>> {
        match character {
            Character::At('>', _) => Some(
                TokenKind::Identifier(std::mem::take(&mut self.identifier)).at(self.start_source),
            ),
            Character::At(c, _) if c.is_alphabetic() || c.is_ascii_digit() || c == '_' => {
                self.identifier.push(c);
                None
            }
            Character::At(c, _) if c.is_whitespace() => Some(
                TokenKind::Error("Whitespace is not allowed inside compound identifiers".into())
                    .at(self.start_source),
            ),
            _ => Some(
                TokenKind::Error("Expected '>' to close compound identifier".into())
                    .at(self.start_source),
            ),
        }
    }
}
