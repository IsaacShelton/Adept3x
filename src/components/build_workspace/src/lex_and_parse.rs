use super::{
    compile::{
        compile_code_file,
        module::{CompiledModule, compile_module_file},
    },
    explore::{ExploreResult, explore},
    file::CodeFile,
    module_file::ModuleFile,
    queue::LexParseQueue,
    stats::CompilationStats,
};
use ast_workspace_settings::Settings;
use compiler::Compiler;
use diagnostics::{ErrorDiagnostic, Show};
use fs_tree::Fs;
use infinite_iterator::InfinitePeekable;
use line_column::Location;
use path_absolutize::Absolutize;
use source_files::{Source, SourceFileKey};
use std::{num::NonZero, sync::Barrier};
use token::Token;

pub fn lex_and_parse_workspace_in_parallel<'a>(
    compiler: &Compiler<'a>,
    fs: &Fs,
    explored: ExploreResult,
    stats: &CompilationStats,
) -> Result<LexParseQueue<'a, impl InfinitePeekable<Token> + 'a>, ()> {
    let ExploreResult {
        normal_files,
        module_files,
    } = explored;

    let source_files = compiler.source_files;
    let thread_count =
        (normal_files.len() + module_files.len()).min(compiler.options.available_parallelism.get());

    let all_modules_done = Barrier::new(thread_count);
    let queue = LexParseQueue::new(normal_files, module_files);

    std::thread::scope(|scope| {
        for _ in 0..thread_count {
            scope.spawn(|| {
                // ===== Process module files =====
                queue.for_module_files(|module_file| {
                    match compile_module_file(compiler, &module_file.path) {
                        Ok(compiled_module) => {
                            process_module_file_output(
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

fn process_module_file_output<'a, I: InfinitePeekable<Token>>(
    compiler: &Compiler,
    fs: &Fs,
    module_file: ModuleFile,
    compiled_module: CompiledModule<'a, I>,
    stats: &CompilationStats,
    queue: &LexParseQueue<'a, I>,
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

    // Queue up any dependencies
    let settings = queue_dependencies(compiler, fs, settings, source_file, stats, queue);

    // Add ourself as a module
    queue.push_module_folder(folder_fs_node_id, settings);
    queue.enqueue_code_file(CodeFile::Module(module_file, remaining_input));

    // Track statistics
    stats.process_file();
    stats.process_bytes(total_file_size);
}

fn queue_dependencies<I: InfinitePeekable<Token>>(
    compiler: &Compiler,
    fs: &Fs,
    mut settings: Settings,
    source_file: SourceFileKey,
    stats: &CompilationStats,
    queue: &LexParseQueue<I>,
) -> Settings {
    let infrastructure = compiler
        .options
        .infrastructure
        .as_ref()
        .expect("must have infrastructure specified in order to import")
        .absolutize()
        .expect("failed to get absolute path for compiler infrastructure");

    let import_folder = infrastructure.join("import");

    for folder in settings.namespace_to_dependency.values().flatten() {
        let absolute_folder_path = import_folder.join(&**folder);
        let already_discovered = fs.find(&absolute_folder_path).is_some();

        if !already_discovered {
            // Find files in dependency codebase
            // NOTE: PERFORMANCE: We should probably have a better way
            // to divide the parallelism or exploring files
            let Ok(new_files) = explore(&fs, &absolute_folder_path, NonZero::new(1).unwrap())
            else {
                ErrorDiagnostic::new(
                    format!("Dependency '{}' could not be found", &**folder),
                    Source::new(source_file, Location::new(0, 1)),
                )
                .eprintln(compiler.source_files);
                stats.fail_module_file();
                return settings;
            };

            // Add the files of the dependency to the queue
            queue.enqueue_module_files(new_files.module_files.into_iter());
            queue.enqueue_code_files(new_files.normal_files.into_iter().map(CodeFile::Normal));
        }

        // Remember where this dependency lives so this module can later use it
        let module_fs_node_id = fs.find(&absolute_folder_path).expect("module loaded");
        settings
            .dependency_to_module
            .insert(folder.to_string(), module_fs_node_id);
    }

    settings
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
        eprintln!(
            "error: No Adept version was specified! Use `pragma => adept(\"3.0\")` at the top of the module file"
        );
        return Err(());
    };

    Ok(())
}
