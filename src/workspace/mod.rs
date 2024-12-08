/*
    ===========================  workspace/mod.rs  ============================
    Module for compiling entire workspaces
    ---------------------------------------------------------------------------
*/

pub mod compile;
mod explore;
pub mod fs;
mod module_file;
mod normal_file;

use crate::{
    ast::{self, AstFile, AstWorkspace, Privacy, Settings},
    c::{
        self,
        lexer::lex_c_code,
        preprocessor::{DefineKind, Preprocessed},
        translate_expr,
    },
    compiler::Compiler,
    data_units::ByteUnits,
    diagnostics::{ErrorDiagnostic, WarningDiagnostic},
    exit_unless,
    inflow::{Inflow, IntoInflow},
    interpreter_env::{run_build_system_interpreter, setup_build_system_interpreter_symbols},
    lexer::Lexer,
    line_column::Location,
    llvm_backend::llvm_backend,
    lower::lower,
    parser::{parse, Input, Parser},
    resolve::resolve,
    show::{into_show, Show},
    source_files::{Source, SourceFileKey},
    text::{IntoText, IntoTextStream, Text},
    token::Token,
};
use append_only_vec::AppendOnlyVec;
use compile::module::{compile_module_file, CompiledModule};
use derive_more::IsVariant;
use explore::{explore, ExploreResult};
use fs::{Fs, FsNodeId};
use indexmap::IndexMap;
use itertools::Itertools;
use module_file::ModuleFile;
use normal_file::{NormalFile, NormalFileKind};
use path_absolutize::Absolutize;
use std::{
    collections::HashMap,
    ffi::OsString,
    fs::create_dir_all,
    path::{Path, PathBuf},
    process::exit,
    sync::{
        atomic::{AtomicU64, Ordering},
        Barrier, Mutex,
    },
    time::Instant,
};
use thousands::Separable;

const NUM_THREADS: usize = 8;

#[derive(IsVariant)]
enum CodeFile<'a, I: Inflow<Token>> {
    Normal(NormalFile),
    Module(ModuleFile, Input<'a, I>),
}

impl<'a, I: Inflow<Token>> CodeFile<'a, I> {
    pub fn path(&self) -> &Path {
        match self {
            CodeFile::Normal(normal_file) => &normal_file.path,
            CodeFile::Module(module_file, _) => &module_file.path,
        }
    }
}

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

struct CompilationStats {
    files_processed: AtomicU64,
    bytes_processed: AtomicU64,
    num_files_failed: AtomicU64,
    num_module_files_failed: AtomicU64,
}

impl CompilationStats {
    pub fn new() -> Self {
        Self {
            files_processed: AtomicU64::new(0),
            bytes_processed: AtomicU64::new(0),
            num_files_failed: AtomicU64::new(0),
            num_module_files_failed: AtomicU64::new(0),
        }
    }

    pub fn failed_files_estimate(&self) -> u64 {
        self.num_files_failed.load(Ordering::Relaxed)
    }

    pub fn failed_modules_estimate(&self) -> u64 {
        self.num_module_files_failed.load(Ordering::Relaxed)
    }

    pub fn bytes_processed_estimate(&self) -> u64 {
        self.bytes_processed.load(Ordering::Relaxed)
    }

    pub fn files_processed_estimate(&self) -> u64 {
        self.files_processed.load(Ordering::Relaxed)
    }

    pub fn fail_file(&self) {
        self.num_files_failed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn fail_module_file(&self) {
        self.num_module_files_failed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn process_file(&self) {
        self.files_processed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn process_bytes(&self, count: ByteUnits) {
        self.bytes_processed
            .fetch_add(count.bytes(), Ordering::Relaxed);
    }
}

struct WorkspaceQueue<'a, I: Inflow<Token>> {
    code_files: Mutex<Vec<CodeFile<'a, I>>>,
    module_files: Mutex<Vec<ModuleFile>>,
    ast_files: AppendOnlyVec<(FsNodeId, AstFile)>,
    module_folders: AppendOnlyVec<(FsNodeId, Settings)>,
}

impl<'a, I: Inflow<Token>> WorkspaceQueue<'a, I> {
    pub fn new(normal_files: Vec<NormalFile>, module_files: Vec<ModuleFile>) -> Self {
        Self {
            code_files: Mutex::new(normal_files.into_iter().map(CodeFile::Normal).collect_vec()),
            module_files: Mutex::new(module_files),
            ast_files: AppendOnlyVec::new(),
            module_folders: AppendOnlyVec::new(),
        }
    }

    pub fn push_module_folder(&self, folder_fs_node_id: FsNodeId, settings: Settings) {
        self.module_folders.push((folder_fs_node_id, settings));
    }

    pub fn push_code_file(&self, code_file: CodeFile<'a, I>) {
        self.code_files.lock().unwrap().push(code_file);
    }

    pub fn push_code_files(&self, code_files: impl Iterator<Item = CodeFile<'a, I>>) {
        self.code_files.lock().unwrap().extend(code_files);
    }

    pub fn push_module_files(&self, module_files: impl Iterator<Item = ModuleFile>) {
        self.module_files.lock().unwrap().extend(module_files);
    }

    pub fn for_module_files(&self, f: impl Fn(ModuleFile)) {
        loop {
            // CAREFUL: Lock doesn't immediately drop unless we do it this way (while loop is not equivalent)
            let Some(module_file) = self.module_files.lock().unwrap().pop() else {
                break;
            };
            f(module_file);
        }
    }

    pub fn for_code_files(&self, f: impl Fn(CodeFile<'a, I>)) {
        loop {
            // CAREFUL: Lock doesn't immediately drop unless we do it this way (while loop is not equivalent)
            let Some(code_file) = self.code_files.lock().unwrap().pop() else {
                break;
            };
            f(code_file);
        }
    }
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
                    let compiled_module =
                        match compile_module_file(compiler, &fs, &module_file.path) {
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

fn compile_code_file<'a, I: Inflow<Token>>(
    compiler: &Compiler,
    code_file: CodeFile<'a, I>,
    out_ast_files: &AppendOnlyVec<(FsNodeId, AstFile)>,
) -> Result<ByteUnits, Box<(dyn Show + 'static)>> {
    match code_file {
        CodeFile::Normal(normal_file) => compile_normal_file(compiler, &normal_file, out_ast_files),
        CodeFile::Module(module_file, rest) => {
            compile_rest_module_file(&module_file, rest, out_ast_files)
        }
    }
}

fn compile_rest_module_file<'a, I: Inflow<Token>>(
    module_file: &ModuleFile,
    rest_input: Input<'a, I>,
    out_ast_files: &AppendOnlyVec<(FsNodeId, AstFile)>,
) -> Result<ByteUnits, Box<(dyn Show + 'static)>> {
    let mut parser = Parser::new(rest_input);
    out_ast_files.push((module_file.fs_node_id, parser.parse().map_err(into_show)?));
    Ok(ByteUnits::ZERO) // No new bytes processed
}

fn compile_normal_file(
    compiler: &Compiler,
    normal_file: &NormalFile,
    out_ast_files: &AppendOnlyVec<(FsNodeId, AstFile)>,
) -> Result<ByteUnits, Box<(dyn Show + 'static)>> {
    let path = &normal_file.path;

    let content = std::fs::read_to_string(path)
        .map_err(ErrorDiagnostic::plain)
        .map_err(into_show)?;

    let source_files = &compiler.source_files;
    let key = source_files.add(path.clone(), content);
    let content = source_files.get(key).content();
    let text = content.chars().into_text_stream(key).into_text();

    match &normal_file.kind {
        NormalFileKind::Adept => {
            out_ast_files.push((
                normal_file.fs_node_id,
                parse(Lexer::new(text).into_inflow(), source_files, key).map_err(into_show)?,
            ));
        }
        NormalFileKind::CSource => {
            compiler.diagnostics.push(WarningDiagnostic::new(
                "c source files are currently treated the same as headers",
                Source::new(key, Location { line: 1, column: 1 }),
            ));

            out_ast_files.push((normal_file.fs_node_id, header(compiler, text, key)?));
        }
        NormalFileKind::CHeader => {
            out_ast_files.push((normal_file.fs_node_id, header(compiler, text, key)?));
        }
    }

    Ok(ByteUnits::of(content.len().try_into().unwrap()))
}

fn header(
    compiler: &Compiler,
    text: impl Text,
    key: SourceFileKey,
) -> Result<AstFile, Box<(dyn Show + 'static)>> {
    let Preprocessed {
        document,
        defines,
        end_of_file,
    } = c::preprocessor::preprocess(text, compiler.diagnostics).map_err(into_show)?;

    let lexed = lex_c_code(document, end_of_file);

    let mut parser = c::parser::Parser::new(
        c::parser::Input::new(lexed, compiler.source_files, key),
        compiler.diagnostics,
    );

    let mut ast_file = parser.parse().map_err(into_show)?;

    // Translate preprocessor #define object macros
    for (define_name, define) in &defines {
        match &define.kind {
            DefineKind::ObjectMacro(expanded_replacement, _placeholder_affinity) => {
                let lexed_replacement =
                    lex_c_code(expanded_replacement.clone(), Source::internal());
                parser.switch_input(lexed_replacement);

                if let Ok(value) = parser.parse_expr_singular().and_then(|expr| {
                    translate_expr(
                        &mut ast_file,
                        parser.typedefs(),
                        &expr,
                        compiler.diagnostics,
                    )
                }) {
                    ast_file.helper_exprs.push(ast::HelperExpr {
                        name: define_name.clone(),
                        value,
                        source: define.source,
                        is_file_local_only: define.is_file_local_only,
                        privacy: Privacy::Public,
                    });
                }
            }
            DefineKind::FunctionMacro(_) => (),
        }
    }

    Ok(ast_file)
}
