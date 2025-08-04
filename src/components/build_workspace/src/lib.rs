/*
    ===========================  workspace/mod.rs  ============================
    Module for compiling entire workspaces
    ---------------------------------------------------------------------------
*/

pub mod compile;
mod explore;
mod explore_within;
mod export_and_link;
mod file;
mod interpreter_env;
mod lex_and_parse;
mod module_file;
mod normal_file;
mod pragma_section;
mod queue;
mod stats;

use ast_workspace::AstWorkspace;
use build_asg::resolve;
use build_ir::lower;
use compiler::Compiler;
use diagnostics::{Show, unerror};
use explore_within::{ExploreWithinResult, explore_within};
use export_and_link::export_and_link;
use fs_tree::Fs;
use interpreter_env::{run_build_system_interpreter, setup_build_system_interpreter_symbols};
use job::{BumpAllocatorPool, TaskState};
use lex_and_parse::lex_and_parse_workspace_in_parallel;
use queue::LexParseInfo;
use stats::CompilationStats;
use std::path::{Path, PathBuf};
use thousands::Separable;

pub fn compile_single_file_only(
    compiler: &mut Compiler,
    project_folder: &Path,
    filepath: &Path,
) -> Result<(), ()> {
    compile_workspace(compiler, project_folder, Some(filepath.to_path_buf()))
}

// Work-in-Progress: New compilation system (full)
pub fn compile(
    compiler: &mut Compiler,
    project_folder: &Path,
    single_file: Option<PathBuf>,
) -> Result<(), ()> {
    let mut allocator = BumpAllocatorPool::new(compiler.options.available_parallelism);
    let build_options = compiler.options.clone();

    let executor = job::MainExecutor::new();
    let main_task = executor.spawn(
        &[],
        job::Main::new(
            &build_options,
            project_folder,
            single_file.as_ref().map(|path_buf| path_buf.as_path()),
        ),
    );

    let executed = executor.start(compiler.source_files, &mut allocator);
    let show_executor_stats = false;

    if executed.errors.len() > 0 {
        for error in executed.errors.iter() {
            error.eprintln(compiler.source_files);
        }
        return Err(());
    } else if executed.num_scheduled != executed.num_completed {
        let num_cyclic = executed.num_scheduled - executed.num_completed;

        if num_cyclic == 1 {
            println!("error: {} cyclic dependency found!", num_cyclic);
        } else {
            println!("error: {} cyclic dependencies found!", num_cyclic);
        }

        for task in executed.truth.tasks.values() {
            if let TaskState::Suspended(_execution, _waiting_count) = &task.state {
                println!(" Incomplete: {:?}", task);
            }
        }
    } else if show_executor_stats {
        println!(
            "Tasks: {}/{}",
            executed.num_completed, executed.num_scheduled,
        );
        println!("Queued: {}/{}", executed.num_cleared, executed.num_queued,);
    }

    if let Some(executable_filepath) = executed.truth.demand(main_task) {
        return compiler.execute_result(&executable_filepath);
    } else {
        Ok(())
    }
}

pub fn compile_workspace(
    compiler: &mut Compiler,
    project_folder: &Path,
    single_file: Option<PathBuf>,
) -> Result<(), ()> {
    // Work-in-Progress: New compilation system (full)
    #[allow(unreachable_code)]
    #[allow(unused_variables)]
    if compiler.options.new_compilation_system.is_full() {
        return compile(compiler, project_folder, single_file);
    }

    let stats = CompilationStats::start();

    let fs = Fs::new();
    let source_files = compiler.source_files;

    // Find workspace files
    let ExploreWithinResult { explored, entry } = explore_within(
        &fs,
        project_folder,
        single_file,
        compiler.options.available_parallelism,
    )
    .map_err(|_| {
        eprintln!("error: Failed to explore workspace folder");
    })?;

    // Lex, parse, apply per-file settings, and bring in dependencies as requested
    let queue = lex_and_parse_workspace_in_parallel(compiler, &fs, explored, &stats)?;

    // Collect lists of all needed ASTs and module folders
    let LexParseInfo {
        module_folders,
        mut files,
    } = queue.destructure();

    // Setup interpreter symbols if requesting to be run in interpreter
    if compiler.options.interpret {
        let Some(guaranteed_entry) = entry else {
            eprintln!(
                "error: Experimental manually-invoked interpreter does not properly handle multiple files yet"
            );
            return Err(());
        };

        setup_build_system_interpreter_symbols(files.get_mut(&guaranteed_entry).unwrap(), false);
    }

    // Compile ASTs into workspace and propagate module settings to each module's contained files
    let workspace = AstWorkspace::new(fs, files, compiler.source_files, module_folders);

    // Resolve symbols and validate semantics for workspace
    let asg = unerror(resolve(&workspace, &compiler.options), source_files)?;

    // Lower code to high level intermediate representation
    let ir_module = unerror(lower(&compiler.options, &asg), source_files)?;

    // Run in interpreter if requesting to be run in interpreter
    if compiler.options.interpret {
        return run_build_system_interpreter(&ir_module)
            .map(|_state| ())
            .map_err(|err| eprintln!("{}", err));
    }

    // Export and link to create executable
    let export_details = export_and_link(compiler, project_folder, &ir_module)?;
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
