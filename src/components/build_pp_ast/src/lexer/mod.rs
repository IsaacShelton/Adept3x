mod lex_line;
mod line;
mod state;

use self::{lex_line::lex_line, state::State};
use super::{error::PreprocessorErrorKind, line_splice::LineSplicer};
use infinite_iterator::InfiniteIterator;
use text::{CharacterPeeker, Text};

// Lexer for C preprocessor
pub struct Lexer<I: Text> {
    state: State,
    line_splicer: LineSplicer<I>,
}

impl<I> Lexer<I>
where
    I: Text,
{
    pub fn new(text: I) -> Self {
        Self {
            state: State::Idle,
            line_splicer: LineSplicer::new(text),
        }
    }
}

pub use self::line::{LexedLine, PreTokenLine};

impl<I> InfiniteIterator for Lexer<I>
where
    I: Text,
{
    type Item = LexedLine;

    fn next(&mut self) -> Self::Item {
        loop {
            match self.line_splicer.next_line() {
                Ok(line) => {
                    let mut line = CharacterPeeker::new(line);

                    if line.peek().is_present() {
                        let start_of_line = line.peek().source();

                        return match lex_line(line, std::mem::take(&mut self.state)) {
                            Ok((tokens, next_state)) => {
                                self.state = next_state;
                                Ok(PreTokenLine::Line(tokens, start_of_line)).into()
                            }
                            Err(err) => return err.into(),
                        };
                    }
                }
                Err(end_of_file_source) => match self.state {
                    State::MultiLineComment(source) => {
                        return PreprocessorErrorKind::UnterminatedMultiLineComment
                            .at(source)
                            .into();
                    }
                    _ => return PreTokenLine::EndOfFile(end_of_file_source).into(),
                },
            }
        }
    }
}
