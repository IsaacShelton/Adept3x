mod error;
mod lexer;
mod number;

use super::{
    preprocessor::{PreToken, PreTokenKind},
    token::CToken,
};
use crate::{
    inflow::{InflowTools, IntoInflow, IntoInflowStream},
    source_files::Source,
};
// Lex errors that will be in tokens if occur
pub use error::LexError;
// The general-purpose C lexer with streaming:
pub use lexer::Lexer;

// Common lexing routine:
// We usually want to convert all of the C preprocessor tokens into C tokens
// at once for each file. This is for a few reasons:
// 1) Our C preprocessor produces a whole document at once (since designed for caching)
// 2) It's much easier to parse C code when you don't do it streaming (since lots of backtracking)
pub fn lex_c_code(preprocessed: Vec<PreToken>, eof_source: Source) -> Vec<CToken> {
    Lexer::new(
        preprocessed
            .into_iter()
            .into_inflow_stream(PreTokenKind::EndOfSequence.at(eof_source))
            .into_inflow(),
    )
    .collect_vec(true)
}
