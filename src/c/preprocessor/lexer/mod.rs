mod lex_line;
mod line;
mod state;

use self::{lex_line::lex_line, state::State};
use super::{error::PreprocessorErrorKind, line_splice::LineSplicer};
use crate::{
    inflow::InflowStream,
    text::{IntoTextNoSend, Text},
};

// Lexer for C preprocessor
pub struct Lexer<T: Text> {
    state: State,
    line_splicer: LineSplicer<T>,
}

impl<T: Text> Lexer<T> {
    pub fn new(text: T) -> Self {
        Self {
            state: State::Idle,
            line_splicer: LineSplicer::new(text),
        }
    }
}

// Output from lexer
pub use self::line::{LexedLine, PreTokenLine};

// The lexer is used via the InflowStream trait
impl<T: Text> InflowStream for Lexer<T> {
    type Item = LexedLine;

    fn next(&mut self) -> Self::Item {
        loop {
            match self.line_splicer.next_line() {
                Ok(line) => {
                    let mut line = line.into_text_no_send();

                    if line.peek().is_present() {
                        let start_of_line = line.peek().source();
                        let (tokens, next_state) = lex_line(line, std::mem::take(&mut self.state))?;
                        self.state = next_state;
                        return Ok(PreTokenLine::Line(tokens, start_of_line));
                    }
                }
                Err(end_of_file_source) => match self.state {
                    State::MultiLineComment(source) => {
                        return Err(PreprocessorErrorKind::UnterminatedMultiLineComment.at(source))
                    }
                    _ => return Ok(PreTokenLine::EndOfFile(end_of_file_source)),
                },
            }
        }
    }
}
