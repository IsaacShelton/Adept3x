mod state;
mod identifier_state;

use crate::{line_column::LineColumn, look_ahead::LookAhead};
use state::State;

pub struct Lexer<I: Iterator<Item = char>> {
    characters: LookAhead<LineColumn<I>>,
    state: State,
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
}
