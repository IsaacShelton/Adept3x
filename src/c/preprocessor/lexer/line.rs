use crate::{
    c::preprocessor::{PreToken, PreprocessorError},
    inflow::InflowEnd,
    source_files::Source,
};
use derive_more::{IsVariant, Unwrap};

#[derive(Clone, Debug, IsVariant, Unwrap)]
pub enum PreTokenLine {
    Line(Vec<PreToken>, Source),
    EndOfFile(Source),
}

pub type LexedLine = Result<PreTokenLine, PreprocessorError>;

impl InflowEnd for LexedLine {
    fn is_inflow_end(&self) -> bool {
        match self {
            Ok(line) => line.is_end_of_file(),
            Err(_) => false,
        }
    }
}
