use source_files::Source;
use token::{Token, TokenKind};

pub struct PolymorphState {
    pub identifier: String,
    pub start_source: Source,
}

impl PolymorphState {
    pub fn finalize(&mut self) -> Token {
        let identifier = std::mem::take(&mut self.identifier);
        TokenKind::Polymorph(identifier).at(self.start_source)
    }
}
