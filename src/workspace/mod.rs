/* ===========================  workspace/mod.rs  ============================
    Module for compiling entire workspaces
    ---------------------------------------------------------------------------
*/

pub mod compile;
mod explore;
mod explore_within;
mod export_and_link;
mod file;
pub mod fs;
mod lex_and_parse;
mod module_file;
mod normal_file;
mod queue;
mod stats;

use crate::{
    ast::AstWorkspace,
    compiler::Compiler,
    interpreter_env::{run_build_system_interpreter, setup_build_system_interpreter_symbols},
    lower::lower,
    resolve::resolve,
    unerror::unerror,
    workspace::export_and_link::export_and_link,
};
use explore_within::{explore_within, ExploreWithinResult};
use fs::Fs;
use indexmap::IndexMap;
use lex_and_parse::lex_and_parse_workspace_in_parallel;
use stats::CompilationStats;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use thousands::Separable;

const NUM_THREADS: usize = 8;

pub fn compile_workspace(
    compiler: &mut Compiler,
    project_folder: &Path,
    single_file: Option<PathBuf>,
) -> Result<(), ()> {
    let stats = CompilationStats::start();

    let fs = Fs::new();
    let source_files = compiler.source_files;

    // Find workspace files
    let ExploreWithinResult { explored, entry } = explore_within(&fs, project_folder, single_file)
        .map_err(|_| {
            eprintln!("error: Failed to explore workspace folder");
        })?;

    // Lex, parse, apply per-file settings, and bring in dependencies as requested
    let queue = lex_and_parse_workspace_in_parallel(compiler, &fs, explored, &stats)?;

    // Collect lists of all needed ASTs and module folders
    let module_folders = HashMap::from_iter(queue.module_folders.into_iter());
    let mut files = IndexMap::from_iter(queue.ast_files.into_iter());

    // Setup interpreter symbols if requesting to be run in interpreter
    if compiler.options.interpret {
        let Some(guaranteed_entry) = entry else {
            eprintln!(
                "error: Experimental manually-invoked interpreter does not properly handle multiple files yet"
            );
            return Err(());
        };

        setup_build_system_interpreter_symbols(files.get_mut(&guaranteed_entry).unwrap());
    }

    // Compile ASTs into workspace and propogate module settings to each module's contained files
    let workspace = AstWorkspace::new(fs, files, compiler.source_files, Some(module_folders));

    // Resolve symbols and validate semantics for workspace
    let resolved_ast = unerror(resolve(&workspace, &compiler.options), source_files)?;

    // Lower code to high level intermediate representation
    let ir_module = unerror(lower(&compiler.options, &resolved_ast), source_files)?;

    // Run in interpreter if requesting to be run in interpreter
    if compiler.options.interpret {
        return run_build_system_interpreter(&resolved_ast, &ir_module)
            .map(|_state| ())
            .map_err(|err| eprintln!("{}", err));
    }

    // Export and link to create executable
    let export_details = export_and_link(compiler, project_folder, &resolved_ast, &ir_module)?;
    print_summary(&stats);
    compiler.execute_result(&export_details.executable_filepath)
}

fn print_summary(stats: &CompilationStats) {
    let in_how_many_seconds = stats.seconds_elapsed();

    // SAFETY: These are okay, as we synchronized by joining
    let files_processed = stats.files_processed_estimate().separate_with_commas();
    let bytes_processed =
        humansize::make_format(humansize::DECIMAL)(stats.bytes_processed_estimate());

    println!(
        "Compiled {} from {} files in {:.2} seconds",
        bytes_processed, files_processed, in_how_many_seconds,
    );
}
