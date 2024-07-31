/*
    ===========================  workspace/mod.rs  ============================
    Module for compiling entire workspaces
    ---------------------------------------------------------------------------
*/

mod explore;
mod fs;
mod normal_file;

use crate::{
    ast::{self, Source},
    c::{
        self,
        lexer::lex_c_code,
        parser::{Input, Parser},
        preprocessor::{DefineKind, Preprocessed},
        translate_expr,
    },
    cli::BuildOptions,
    compiler::Compiler,
    diagnostics::WarningDiagnostic,
    exit_unless,
    inflow::{Inflow, InflowStream, IntoInflow},
    interpreter_env::{run_build_system_interpreter, setup_build_system_interpreter_symbols},
    lexer::Lexer,
    llvm_backend::llvm_backend,
    lower::lower,
    parser::{parse, parse_into},
    resolve::resolve,
    show::error_println,
    text::{IntoText, IntoTextStream, TextStream},
    token::{Token, TokenKind},
};
use explore::{explore, ExploreResult};
use fs::FsNode;
use normal_file::NormalFile;
use std::{
    ffi::OsStr,
    path::Path,
    process::exit,
    sync::{
        atomic::{AtomicU64, Ordering},
        Barrier, Mutex,
    },
    time::Instant,
};
use thousands::Separable;
use walkdir::{DirEntry, WalkDir};

const NUM_THREADS: usize = 8;

pub fn compile_workspace(compiler: &Compiler, folder_path: &Path) {
    let compiler = compiler;

    let start_time = Instant::now();
    let files_processed = AtomicU64::new(0);
    let bytes_processed = AtomicU64::new(0);
    let files_failed = AtomicU64::new(0);

    let ExploreResult {
        root_node,
        module_files,
        normal_files,
    } = explore(folder_path);

    let num_threads = (normal_files.len() + module_files.len()).min(NUM_THREADS);
    let all_modules_done = Barrier::new(num_threads);
    let normal_files = Mutex::new(normal_files);
    let module_files = Mutex::new(module_files);

    std::thread::scope(|scope| {
        for _ in 0..num_threads {
            scope.spawn(|| {
                // ===== Process module files =====
                while let Some(module_file) = module_files.lock().unwrap().pop() {
                    let Ok(did_bytes) = compile_module_file(compiler, &root_node, &module_file)
                    else {
                        files_failed.fetch_add(1, Ordering::Relaxed);
                        continue;
                    };

                    files_processed.fetch_add(1, Ordering::Relaxed);
                    bytes_processed.fetch_add(did_bytes.try_into().unwrap(), Ordering::Relaxed);
                }

                all_modules_done.wait();

                // ===== Process normal files =====
                while let Some(normal_file) = normal_files.lock().unwrap().pop() {
                    let Ok(did_bytes) = compile_normal_file(compiler, &root_node, &normal_file)
                    else {
                        files_failed.fetch_add(1, Ordering::Relaxed);
                        continue;
                    };

                    files_processed.fetch_add(1, Ordering::Relaxed);
                    bytes_processed.fetch_add(did_bytes.try_into().unwrap(), Ordering::Relaxed);
                }
            });
        }
    });

    if true {
        compiler.diagnostics.push(WarningDiagnostic::plain(
            "Module system is not fully implemented yet, falling back to old system",
        ));
        return old_compile_workspace(compiler, folder_path);
    }

    // Print summary:

    let in_how_many_seconds = start_time.elapsed().as_millis() as f64 / 1000.0;
    let files_failed = files_failed.load(Ordering::SeqCst);

    if files_failed != 0 {
        eprintln!(
            "error: {} files were determined to have errors in {:.2} seconds",
            files_failed, in_how_many_seconds
        );

        exit(1);
    }

    let bytes_processed =
        humansize::make_format(humansize::DECIMAL)(bytes_processed.load(Ordering::SeqCst));

    let files_processed = files_processed
        .load(Ordering::SeqCst)
        .separate_with_commas();

    println!(
        "Compiled {} from {} files in {:.2} seconds",
        bytes_processed, files_processed, in_how_many_seconds
    );
}

fn compile_module_file(compiler: &Compiler, _root_node: &FsNode, path: &Path) -> Result<usize, ()> {
    println!("[module] {}", path.display());

    let content = std::fs::read_to_string(path).map_err(|err| {
        eprintln!("{}", err);
        ()
    })?;

    let source_file_cache = &compiler.source_file_cache;
    let key = source_file_cache.add(path.to_path_buf(), content);
    let content = source_file_cache.get(key).content();

    let text = content.chars().into_text_stream(key).into_text();
    let mut lexer = Lexer::new(text).into_inflow();

    // Ignore initial newlines
    while lexer.peek().kind.is_newline() {
        lexer.next();
    }

    match lexer.next() {
        Token {
            kind: TokenKind::PragmaKeyword,
            source,
        } => {
            // TODO: UNIMPLEMENTED: Implement pragma sections
            if true {
                error_println(
                    "Handling pragma sections not implemented yet",
                    source,
                    compiler.source_file_cache,
                );
                return Err(());
            }
        }
        bad => {
            error_println(
                "Expected 'pragma' at beginning of module file",
                bad.source,
                compiler.source_file_cache,
            );
            return Err(());
        }
    }

    let mut ast = parse(lexer, source_file_cache, key).map_err(|_| ())?;

    let compiler = Compiler {
        options: BuildOptions {
            emit_llvm_ir: false,
            emit_ir: false,
            interpret: true,
            coerce_main_signature: false,
        },
        target_info: compiler.target_info.clone(),
        source_file_cache: compiler.source_file_cache,
        diagnostics: compiler.diagnostics,
    };

    setup_build_system_interpreter_symbols(&mut ast);

    let resolved_ast = resolve(&ast, &compiler.options).map_err(|_| ())?;

    let ir_module =
        lower(&compiler.options, &resolved_ast, &compiler.target_info).map_err(|_| ())?;

    run_build_system_interpreter(&resolved_ast, &ir_module);

    Ok(content.len())
}

fn compile_normal_file(
    compiler: &Compiler,
    _root_node: &FsNode,
    normal_file: &NormalFile,
) -> Result<usize, ()> {
    let path = &normal_file.path;

    println!("[code] {}", path.display());

    let content = std::fs::read_to_string(path).map_err(|err| {
        eprintln!("{}", err);
        ()
    })?;

    let source_file_cache = &compiler.source_file_cache;
    let key = source_file_cache.add(path.clone(), content);
    let content = source_file_cache.get(key).content();

    Ok(content.len())
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with("."))
        .unwrap_or(false)
}

pub fn old_compile_workspace(compiler: &Compiler, folder_path: &Path) {
    let source_file_cache = &compiler.source_file_cache;

    let walker = WalkDir::new(folder_path).min_depth(1).into_iter();
    let mut ast = None;

    let output_binary_filepath = folder_path.join("a.out");
    let output_object_filepath = folder_path.join("a.o");

    for entry in walker.filter_entry(|e| !is_hidden(e)) {
        let entry = entry.expect("walk dir");
        let extension = entry.path().extension();

        let is_header = match extension.and_then(OsStr::to_str) {
            Some("adept") => false,
            Some("h") => true,
            _ => continue,
        };

        let filepath = entry.path();
        let filename = filepath.to_string_lossy().to_string();
        println!("[=] {filename}");

        let content = std::fs::read_to_string(&filename)
            .map_err(|err| {
                eprintln!("{}", err);
                exit(1);
            })
            .unwrap();

        let key = source_file_cache.add(filepath.into(), content);
        let content = source_file_cache.get(key).content();

        if !is_header {
            let lexer = Lexer::new(content.chars().into_text(key)).into_inflow();

            if let Some(ast) = &mut ast {
                exit_unless(
                    parse_into(lexer, source_file_cache, key, ast, filename.to_string()),
                    source_file_cache,
                );
            } else {
                ast = Some(exit_unless(
                    parse(lexer, source_file_cache, key),
                    source_file_cache,
                ));
            }
        } else {
            let Preprocessed {
                document,
                defines,
                end_of_file,
            } = exit_unless(
                c::preprocessor::preprocess(content.chars().into_text(key), compiler.diagnostics),
                source_file_cache,
            );

            let lexed = lex_c_code(document, end_of_file);

            let mut parser = Parser::new(
                Input::new(lexed, source_file_cache, key),
                compiler.diagnostics,
            );

            let file_id = if let Some(ast) = &mut ast {
                exit_unless(parser.parse_into(ast, filename), source_file_cache)
            } else {
                let (new_ast, file_id) = exit_unless(parser.parse(), source_file_cache);
                ast = Some(new_ast);
                file_id
            };

            // Translate preprocessor #define object macros
            let ast_file = ast
                .as_mut()
                .expect("ast to exist")
                .files
                .get_mut(&file_id)
                .expect("recently added file to exist");

            for (define_name, define) in &defines {
                match &define.kind {
                    DefineKind::ObjectMacro(expanded_replacement, _placeholder_affinity) => {
                        let lexed_replacement =
                            lex_c_code(expanded_replacement.clone(), Source::internal());
                        parser.switch_input(lexed_replacement);

                        if let Ok(value) = parser.parse_expr_singular().and_then(|expr| {
                            translate_expr(ast_file, parser.typedefs(), &expr, compiler.diagnostics)
                        }) {
                            ast_file.defines.insert(
                                define_name.clone(),
                                ast::Define {
                                    value,
                                    source: define.source,
                                },
                            );
                        }
                    }
                    DefineKind::FunctionMacro(_) => (),
                }
            }
        }
    }

    let Some(ast) = ast else {
        eprintln!("must have at least one adept file in directory in order to compile");
        exit(1);
    };

    let resolved_ast = exit_unless(resolve(&ast, &compiler.options), source_file_cache);

    let ir_module = exit_unless(
        lower(&compiler.options, &resolved_ast, &compiler.target_info),
        source_file_cache,
    );

    exit_unless(
        unsafe {
            llvm_backend(
                &compiler.options,
                &ir_module,
                &resolved_ast,
                &output_object_filepath,
                &output_binary_filepath,
                &compiler.diagnostics,
            )
        },
        source_file_cache,
    );
}
