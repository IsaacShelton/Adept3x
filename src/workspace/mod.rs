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
pub mod fs;
mod module_file;
mod normal_file;
mod queue;
mod stats;

use crate::{
    ast::{AstWorkspace, Settings},
    compiler::Compiler,
    diagnostics::ErrorDiagnostic,
    inflow::Inflow,
    interpreter_env::{run_build_system_interpreter, setup_build_system_interpreter_symbols},
    line_column::Location,
    lower::lower,
    resolve::resolve,
    show::Show,
    source_files::{Source, SourceFileKey},
    token::Token,
    unerror::unerror,
    workspace::export_and_link::export_and_link,
};
use compile::{
    compile_code_file,
    module::{compile_module_file, CompiledModule},
};
use explore::{explore, ExploreResult};
use explore_within::{explore_within, ExploreWithinResult};
use file::CodeFile;
use fs::Fs;
use indexmap::IndexMap;
use module_file::ModuleFile;
use path_absolutize::Absolutize;
use queue::WorkspaceQueue;
use stats::CompilationStats;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Barrier,
};
use thousands::Separable;

const NUM_THREADS: usize = 8;

fn lex_and_parse_workspace_in_parallel<'a>(
    compiler: &Compiler<'a>,
    fs: &Fs,
    explored: ExploreResult,
    stats: &CompilationStats,
) -> Result<WorkspaceQueue<'a, impl Inflow<Token> + 'a>, ()> {
    let ExploreResult {
        normal_files,
        module_files,
    } = explored;

    let source_files = compiler.source_files;
    let thread_count = (normal_files.len() + module_files.len()).min(NUM_THREADS);

    let all_modules_done = Barrier::new(thread_count);
    let queue = WorkspaceQueue::new(normal_files, module_files);

    std::thread::scope(|scope| {
        for _ in 0..thread_count {
            scope.spawn(|| {
                // ===== Process module files =====
                queue.for_module_files(|module_file| {
                    match compile_module_file(compiler, &module_file.path) {
                        Ok(compiled_module) => {
                            process_module_file(
                                compiler,
                                &fs,
                                module_file,
                                compiled_module,
                                &stats,
                                &queue,
                            );
                        }
                        Err(failed_module_message) => {
                            failed_module_message.eprintln(source_files);
                            stats.fail_module_file();
                        }
                    }
                });

                // NOTE: This synchronizes the threads, and marks the end of module-related modifications/processing
                all_modules_done.wait();

                // ==== Don't continue if module files had errors =====
                // SAFETY: This is okay, as all the modifications happen before synchronizing the modifying threads
                if stats.failed_modules_estimate() != 0 {
                    return;
                }

                // ===== Process normal files =====
                queue.for_code_files(|code_file| {
                    match compile_code_file(compiler, code_file, &queue.ast_files) {
                        Ok(did_bytes) => {
                            stats.process_file();
                            stats.process_bytes(did_bytes);
                        }
                        Err(error_message) => {
                            error_message.eprintln(source_files);
                            stats.fail_file();
                        }
                    };
                });
            });
        }
    });

    print_syntax_errors(compiler, &stats)?;
    Ok(queue)
}

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

    // Compile ASTs into workspace and propogate module setings to each module's contained files
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

fn process_module_file<'a, 'b: 'a, I: Inflow<Token>>(
    compiler: &Compiler,
    fs: &Fs,
    module_file: ModuleFile,
    compiled_module: CompiledModule<'a, I>,
    stats: &CompilationStats,
    queue: &WorkspaceQueue<'a, I>,
) {
    let folder_fs_node_id = fs
        .get(module_file.fs_node_id)
        .parent
        .expect("module file has parent");

    let CompiledModule {
        settings,
        source_file,
        total_file_size,
        remaining_input,
    } = compiled_module;

    let settings = queue_dependencies(compiler, fs, settings, source_file, stats, queue);

    queue.push_module_folder(folder_fs_node_id, settings);
    queue.push_code_file(CodeFile::Module(module_file, remaining_input));

    stats.process_file();
    stats.process_bytes(total_file_size);
}

fn queue_dependencies<I: Inflow<Token>>(
    compiler: &Compiler,
    fs: &Fs,
    mut settings: Settings,
    source_file: SourceFileKey,
    stats: &CompilationStats,
    queue: &WorkspaceQueue<I>,
) -> Settings {
    for folder in settings.namespace_to_dependency.values().flatten() {
        let infrastructure = compiler
            .options
            .infrastructure
            .as_ref()
            .expect("must have infrastructure specified in order to import")
            .absolutize()
            .expect("failed to get absolute path for compiler infrastructure");

        let absolute_folder = infrastructure.join("import").join(&**folder);
        let already_discovered = fs.find(&absolute_folder).is_some();

        if !already_discovered {
            let Ok(ExploreResult {
                module_files: new_module_files,
                normal_files: new_normal_files,
            }) = explore(&fs, &absolute_folder)
            else {
                ErrorDiagnostic::new(
                    format!("Dependency '{}' could not be found", &**folder),
                    Source::new(source_file, Location::new(0, 1)),
                )
                .eprintln(compiler.source_files);
                stats.fail_module_file();
                return settings;
            };

            queue.push_module_files(new_module_files.into_iter());
            queue.push_code_files(new_normal_files.into_iter().map(CodeFile::Normal));
        }

        let module_fs_node_id = fs.find(&absolute_folder).expect("module loaded");
        settings
            .dependency_to_module
            .insert(folder.to_string(), module_fs_node_id);
    }

    settings
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

fn print_syntax_errors(compiler: &Compiler, stats: &CompilationStats) -> Result<(), ()> {
    let in_how_many_seconds = stats.seconds_elapsed();

    // SAFETY: This is okay since all modifying threads were joined (and thereby synchronized)
    let num_module_files_failed = stats.failed_modules_estimate();
    if num_module_files_failed != 0 {
        eprintln!(
            "error: {num_module_files_failed} module file(s) were determined to have errors in {in_how_many_seconds:.2} seconds",
        );
        return Err(());
    }

    // SAFETY: This is okay since all modifying threads were joined (and thereby synchronized)
    let num_files_failed = stats.failed_files_estimate();
    if num_files_failed != 0 {
        eprintln!(
            "error: {num_files_failed} file(s) were determined to have errors in {in_how_many_seconds:.2} seconds",
        );
        return Err(());
    }

    let Some(_adept_version) = compiler.version.get() else {
        eprintln!("error: No Adept version was specified! Use `pragma => adept(\"3.0\")` at the top of the module file");
        return Err(());
    };

    Ok(())
}
