/*
    ===========================  workspace/mod.rs  ============================
    Module for compiling entire workspaces
    ---------------------------------------------------------------------------
*/

pub mod compile;
mod explore;
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
    exit_unless,
    inflow::Inflow,
    interpreter_env::{run_build_system_interpreter, setup_build_system_interpreter_symbols},
    line_column::Location,
    llvm_backend::llvm_backend,
    lower::lower,
    resolve::resolve,
    show::Show,
    source_files::{Source, SourceFileKey},
    token::Token,
};
use compile::{
    compile_code_file,
    module::{compile_module_file, CompiledModule},
};
use explore::{explore, ExploreResult};
use file::CodeFile;
use fs::{Fs, FsNodeId};
use indexmap::IndexMap;
use module_file::ModuleFile;
use path_absolutize::Absolutize;
use queue::WorkspaceQueue;
use stats::CompilationStats;
use std::{
    collections::HashMap,
    ffi::OsString,
    fs::create_dir_all,
    path::{Path, PathBuf},
    process::exit,
    sync::Barrier,
    time::Instant,
};
use thousands::Separable;

const NUM_THREADS: usize = 8;

fn explore_constrained(
    fs: &Fs,
    project_folder: &Path,
    single_file: Option<PathBuf>,
) -> (Option<ExploreResult>, Option<FsNodeId>) {
    if let Some(single_file) = single_file {
        let fs_node_id = fs.insert(&single_file, None).expect("inserted");

        let file = ModuleFile {
            path: single_file,
            fs_node_id,
        };

        return (
            Some(ExploreResult {
                normal_files: vec![],
                module_files: vec![file],
            }),
            Some(fs_node_id),
        );
    }

    (explore(&fs, project_folder), None)
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
            let Some(ExploreResult {
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

pub fn compile_workspace(
    compiler: &mut Compiler,
    project_folder: &Path,
    single_file: Option<PathBuf>,
) {
    let start_time = Instant::now();
    let stats = CompilationStats::new();

    let fs = Fs::new();
    let (exploration, guaranteed_entry) = explore_constrained(&fs, project_folder, single_file);

    let Some(ExploreResult {
        module_files,
        normal_files,
    }) = exploration
    else {
        eprintln!(
            "error: Could not locate workspace folder '{}'",
            project_folder.display()
        );
        exit(1);
    };

    let thread_count = (normal_files.len() + module_files.len()).min(NUM_THREADS);
    let all_modules_done = Barrier::new(thread_count);
    let queue = WorkspaceQueue::new(normal_files, module_files);

    std::thread::scope(|scope| {
        for _ in 0..thread_count {
            scope.spawn(|| {
                // ===== Process module files =====
                queue.for_module_files(|module_file| {
                    let compiled_module = match compile_module_file(compiler, &module_file.path) {
                        Ok(values) => values,
                        Err(err) => {
                            err.eprintln(compiler.source_files);
                            stats.fail_module_file();
                            return;
                        }
                    };

                    process_module_file(
                        compiler,
                        &fs,
                        module_file,
                        compiled_module,
                        &stats,
                        &queue,
                    );
                });

                // NOTE: This synchronizes the threads, and marks the end of module-related modifications/processing.
                // `num_module_files_failed` can now be consistently read from...
                all_modules_done.wait();

                // ==== Don't continue if module files had errors =====
                // SAFETY: This is okay, as all the modifications happened before we synchronized
                // the modifying threads.
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
                        Err(err) => {
                            err.eprintln(compiler.source_files);
                            stats.fail_file();
                        }
                    };
                });
            });
        }
    });

    let in_how_many_seconds = start_time.elapsed().as_millis() as f64 / 1000.0;

    // SAFETY: This is okay since all modifying threads were joined (and thereby synchronized)
    let num_module_files_failed = stats.failed_modules_estimate();
    if num_module_files_failed != 0 {
        eprintln!(
            "error: {num_module_files_failed} module file(s) were determined to have errors in {in_how_many_seconds:.2} seconds",
        );

        exit(1);
    }

    // SAFETY: This is okay since all modifying threads were joined (and thereby synchronized)
    let num_files_failed = stats.failed_files_estimate();
    if num_files_failed != 0 {
        eprintln!(
            "error: {num_files_failed} file(s) were determined to have errors in {in_how_many_seconds:.2} seconds",
        );

        exit(1);
    }

    let Some(_adept_version) = compiler.version.get() else {
        eprintln!("error: No Adept version was specified! Use `pragma => adept(\"3.0\")` at the top of the module file");
        exit(1);
    };

    let module_folders = HashMap::<FsNodeId, Settings>::from_iter(queue.module_folders.into_iter());
    let mut files = IndexMap::from_iter(queue.ast_files.into_iter());

    if compiler.options.interpret {
        if let Some(guaranteed_entry) = guaranteed_entry {
            setup_build_system_interpreter_symbols(files.get_mut(&guaranteed_entry).unwrap());
        } else {
            eprintln!(
                "error: experimental manual interpreter does not properly handle multiple files yet"
            );
            exit(1);
        }
    }

    let workspace = AstWorkspace::new(fs, files, compiler.source_files, Some(module_folders));

    let resolved_ast = exit_unless(
        resolve(&workspace, &compiler.options),
        compiler.source_files,
    );

    let ir_module = exit_unless(
        lower(&compiler.options, &resolved_ast, &compiler.target),
        compiler.source_files,
    );

    let project_name = project_folder
        .file_name()
        .map(OsString::from)
        .unwrap_or_else(|| {
            std::env::current_dir()
                .ok()
                .map(|dir| {
                    dir.file_name()
                        .map(OsString::from)
                        .unwrap_or_else(|| OsString::from("main"))
                })
                .unwrap_or_else(|| OsString::from("main"))
        });

    if compiler.options.interpret {
        match run_build_system_interpreter(&resolved_ast, &ir_module) {
            Ok(_) => return,
            Err(err) => {
                eprintln!("{}", err);
                exit(1);
            }
        }
    }

    let bin_folder = project_folder.join("bin");
    let obj_folder = project_folder.join("obj");

    create_dir_all(&bin_folder).expect("failed to create bin folder");
    create_dir_all(&obj_folder).expect("failed to create obj folder");

    let exe_filepath = bin_folder.join(compiler.target.default_executable_name(&project_name));
    let obj_filepath = obj_folder.join(compiler.target.default_object_file_name(&project_name));

    let linking_duration = exit_unless(
        unsafe {
            llvm_backend(
                compiler,
                &ir_module,
                &resolved_ast,
                &obj_filepath,
                &exe_filepath,
                &compiler.diagnostics,
            )
        },
        compiler.source_files,
    );

    // Print summary:

    let in_how_many_seconds = start_time.elapsed().as_millis() as f64 / 1000.0;
    let _linking_took = linking_duration.as_millis() as f64 / 1000.0;

    // SAFETY: This is okay, as we synchronized by joining
    let bytes_processed =
        humansize::make_format(humansize::DECIMAL)(stats.bytes_processed_estimate());

    // SAFETY: This is okay, as we synchronized by joining
    let files_processed = stats.files_processed_estimate().separate_with_commas();

    println!(
        "Compiled {} from {} files in {:.2} seconds",
        bytes_processed, files_processed, in_how_many_seconds,
    );

    compiler.maybe_execute_result(&exe_filepath);
}
