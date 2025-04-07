use crate::error::PreprocessorError;
use derive_more::{From, IsVariant, Unwrap};
use infinite_iterator::InfiniteIteratorEnd;
use pp_token::PreToken;
use source_files::Source;

#[derive(Clone, Debug, IsVariant, Unwrap)]
pub enum PreTokenLine {
    Line(Vec<PreToken>, Source),
    EndOfFile(Source),
}

#[derive(Clone, Debug, From)]
pub struct LexedLine(Result<PreTokenLine, PreprocessorError>);

impl LexedLine {
    pub fn result(self) -> Result<PreTokenLine, PreprocessorError> {
        self.0
    }

    pub fn unwrap(self) -> PreTokenLine {
        self.0.unwrap()
    }

    pub fn expect(self, msg: &'static str) -> PreTokenLine {
        self.0.expect(msg)
    }

    pub fn as_ref(&self) -> Result<&PreTokenLine, &PreprocessorError> {
        self.0.as_ref()
    }

    pub fn as_ok_ref(&self) -> Result<&PreTokenLine, PreprocessorError> {
        self.0.as_ref().map_err(Clone::clone)
    }
}

impl InfiniteIteratorEnd for LexedLine {
    fn is_end(&self) -> bool {
        match &self.0 {
            Ok(line) => line.is_end_of_file(),
            Err(_) => false,
        }
    }
}

impl From<PreTokenLine> for LexedLine {
    fn from(value: PreTokenLine) -> Self {
        Self(Ok(value))
    }
}

impl From<PreprocessorError> for LexedLine {
    fn from(value: PreprocessorError) -> Self {
        Self(Err(value))
    }
}
