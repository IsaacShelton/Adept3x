#![allow(dead_code)]
#![allow(clippy::diverging_sub_expression)]
#![allow(clippy::module_name_repetitions)]
#![feature(string_remove_matches)]
#![feature(never_type)]
#![feature(exhaustive_patterns)]
#![feature(maybe_uninit_array_assume_init)]
#![feature(once_cell_try_insert)]

mod asg;
mod ast;
mod backend;
mod borrow;
mod c;
mod cli;
mod compiler;
mod data_units;
mod diagnostics;
mod hash_map_ext;
mod index_map_ext;
mod inflow;
mod interpreter;
mod interpreter_env;
mod ir;
mod iter_ext;
mod lexer;
mod lexical_utils;
mod line_column;
mod linking;
mod llvm_backend;
mod logic;
mod look_ahead;
mod lower;
mod name;
mod parser;
mod path;
mod pragma_section;
mod repeating_last;
mod resolve;
mod show;
mod single_file_only;
mod source_files;
mod tag;
mod target;
mod text;
mod token;
mod unerror;
mod version;
mod workspace;

use cli::{CliCommand, CliInvoke};
use std::process::ExitCode;

fn main() -> ExitCode {
    match CliCommand::parse().and_then(CliInvoke::invoke) {
        Ok(()) => ExitCode::SUCCESS,
        Err(()) => ExitCode::FAILURE,
    }
}
