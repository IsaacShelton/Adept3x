#![feature(string_remove_matches)]

mod lexer;
mod number;

use c_token::CToken;
pub use c_token::Invalid as LexError;
use inflow::{InflowTools, IntoInflow, IntoInflowStream};
pub use lexer::Lexer;
use pp_token::{PreToken, PreTokenKind};
use source_files::Source;

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
