/*
    ===========================  workspace/mod.rs  ============================
    Module for compiling entire workspaces
    ---------------------------------------------------------------------------
*/

mod compile;
mod explore;
pub mod fs;
mod module_file;
mod normal_file;

use crate::{
    ast::{self, AstFile, AstWorkspace, Settings},
    c::{
        self,
        lexer::lex_c_code,
        preprocessor::{DefineKind, Preprocessed},
        translate_expr,
    },
    compiler::Compiler,
    diagnostics::{ErrorDiagnostic, WarningDiagnostic},
    exit_unless,
    inflow::{Inflow, IntoInflow},
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
use compile::module::compile_module_file;
use derive_more::IsVariant;
use explore::{explore, ExploreResult};
use fs::{Fs, FsNodeId};
use indexmap::IndexMap;
use itertools::Itertools;
use module_file::ModuleFile;
use normal_file::{NormalFile, NormalFileKind};
use std::{
    collections::HashMap,
    path::Path,
    process::exit,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
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

pub fn compile_workspace(compiler: &mut Compiler, folder_path: &Path) {
    let compiler = compiler;

    let start_time = Instant::now();
    let files_processed = AtomicU64::new(0);
    let bytes_processed = AtomicU64::new(0);
    let num_files_failed = AtomicU64::new(0);

    let fs = Fs::new();

    let ExploreResult {
        module_files,
        mut normal_files,
    } = explore(&fs, folder_path);

    let num_threads = (normal_files.len() + module_files.len()).min(NUM_THREADS);
    let all_modules_done = Barrier::new(num_threads);
    let code_files = Mutex::new(normal_files.drain(..).map(CodeFile::Normal).collect_vec());
    let module_files = Mutex::new(module_files);
    let has_module_errors = AtomicBool::new(false);
    let ast_files = AppendOnlyVec::new();
    let module_folders = AppendOnlyVec::new();

    std::thread::scope(|scope| {
        for _ in 0..num_threads {
            scope.spawn(|| {
                // ===== Process module files =====
                loop {
                    // CAREFUL: Lock doesn't immediately drop unless we do it this way (while loop is not equivalent)
                    let Some(module_file) = module_files.lock().unwrap().pop() else {
                        break;
                    };

                    let (did_bytes, rest_input, settings) =
                        match compile_module_file(compiler, &fs, &module_file.path) {
                            Ok(values) => values,
                            Err(err) => {
                                let mut message = String::new();
                                err.show(&mut message, compiler.source_files)
                                    .expect("failed to print error");
                                eprintln!("{}", message);

                                num_files_failed.fetch_add(1, Ordering::Relaxed);
                                continue;
                            }
                        };

                    let folder_fs_node_id = fs
                        .get(module_file.fs_node_id)
                        .parent
                        .expect("module file as parent");

                    module_folders.push((folder_fs_node_id, settings));

                    code_files
                        .lock()
                        .unwrap()
                        .push(CodeFile::Module(module_file, rest_input));

                    files_processed.fetch_add(1, Ordering::Relaxed);
                    bytes_processed.fetch_add(did_bytes.try_into().unwrap(), Ordering::Relaxed);
                }

                all_modules_done.wait();

                // ==== Don't continue if module files had errors =====
                if num_files_failed.load(Ordering::SeqCst) != 0 {
                    has_module_errors.store(true, Ordering::Relaxed);
                    return;
                }

                // ===== Process normal files =====
                loop {
                    // CAREFUL: Lock doesn't immediately drop unless we do it this way (while loop is not equivalent)
                    let Some(code_file) = code_files.lock().unwrap().pop() else {
                        break;
                    };

                    let did_bytes = match compile_code_file(compiler, code_file, &ast_files) {
                        Ok(did_bytes) => did_bytes,
                        Err(err) => {
                            let mut message = String::new();
                            err.show(&mut message, compiler.source_files)
                                .expect("failed to print error");
                            eprintln!("{}", message);

                            num_files_failed.fetch_add(1, Ordering::Relaxed);
                            continue;
                        }
                    };

                    files_processed.fetch_add(1, Ordering::Relaxed);
                    bytes_processed.fetch_add(did_bytes.try_into().unwrap(), Ordering::Relaxed);
                }
            });
        }
    });

    let in_how_many_seconds = start_time.elapsed().as_millis() as f64 / 1000.0;
    let num_files_failed = num_files_failed.load(Ordering::SeqCst);

    if num_files_failed != 0 {
        let prefix = if has_module_errors.load(Ordering::SeqCst) {
            "module "
        } else {
            ""
        };

        eprintln!(
            "error: {num_files_failed} {prefix}file(s) were determined to have errors in {in_how_many_seconds:.2} seconds",
        );

        exit(1);
    }

    let Some(_adept_version) = compiler.version.get() else {
        eprintln!("error: No Adept version was specified!, Use `pragma => adept(c\"3.0\")` at the top of your module file");
        exit(1);
    };

    let module_folders = HashMap::<FsNodeId, Settings>::from_iter(module_folders.into_iter());
    let files = IndexMap::from_iter(ast_files.into_iter());
    let workspace = AstWorkspace::new(fs, files, compiler.source_files, Some(module_folders));

    let resolved_ast = exit_unless(
        resolve(&workspace, &compiler.options),
        compiler.source_files,
    );

    let ir_module = exit_unless(
        lower(&compiler.options, &resolved_ast, &compiler.target_info),
        compiler.source_files,
    );

    let project_folder = folder_path;
    let output_binary_filepath = project_folder.join("a.out");
    let output_object_filepath = project_folder.join("a.o");

    let linking_duration = exit_unless(
        unsafe {
            llvm_backend(
                compiler,
                &ir_module,
                &resolved_ast,
                &output_object_filepath,
                &output_binary_filepath,
                &compiler.diagnostics,
            )
        },
        compiler.source_files,
    );

    // Print summary:

    let in_how_many_seconds = start_time.elapsed().as_millis() as f64 / 1000.0;
    let _linking_took = linking_duration.as_millis() as f64 / 1000.0;

    let bytes_processed =
        humansize::make_format(humansize::DECIMAL)(bytes_processed.load(Ordering::SeqCst));

    let files_processed = files_processed
        .load(Ordering::SeqCst)
        .separate_with_commas();

    println!(
        "Compiled {} from {} files in {:.2} seconds",
        bytes_processed, files_processed, in_how_many_seconds,
    );

    compiler.maybe_execute_result(output_binary_filepath.as_os_str());
}

fn compile_code_file<'a, I: Inflow<Token>>(
    compiler: &Compiler,
    code_file: CodeFile<'a, I>,
    out_ast_files: &AppendOnlyVec<(FsNodeId, AstFile)>,
) -> Result<usize, Box<(dyn Show + 'static)>> {
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
) -> Result<usize, Box<(dyn Show + 'static)>> {
    let mut parser = Parser::new(rest_input);
    out_ast_files.push((module_file.fs_node_id, parser.parse().map_err(into_show)?));
    Ok(0) // No new bytes processed
}

fn compile_normal_file(
    compiler: &Compiler,
    normal_file: &NormalFile,
    out_ast_files: &AppendOnlyVec<(FsNodeId, AstFile)>,
) -> Result<usize, Box<(dyn Show + 'static)>> {
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

    Ok(content.len())
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
                    ast_file.helper_exprs.insert(
                        define_name.clone(),
                        ast::HelperExpr {
                            value,
                            source: define.source,
                            is_file_local_only: define.is_file_local_only,
                        },
                    );
                }
            }
            DefineKind::FunctionMacro(_) => (),
        }
    }

    Ok(ast_file)
}
