use token::{Token, TokenKind};

pub struct PolymorphState<S: Copy> {
    pub identifier: String,
    pub start_source: S,
}

impl<S: Copy> PolymorphState<S> {
    pub fn finalize(&mut self) -> Token<S> {
        let identifier = std::mem::take(&mut self.identifier);
        TokenKind::Polymorph(identifier).at(self.start_source)
    }
}
