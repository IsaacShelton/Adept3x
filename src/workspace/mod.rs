/*
    ===========================  workspace/mod.rs  ============================
    Module for compiling entire workspaces
    ---------------------------------------------------------------------------
*/

mod compile;
mod explore;
pub mod fs;
mod normal_file;

use crate::{
    ast,
    c::{
        self,
        lexer::lex_c_code,
        preprocessor::{DefineKind, Preprocessed},
        translate_expr,
    },
    compiler::Compiler,
    diagnostics::WarningDiagnostic,
    exit_unless,
    inflow::IntoInflow,
    lexer::Lexer,
    llvm_backend::llvm_backend,
    lower::lower,
    parser::{parse, parse_into},
    resolve::resolve,
    source_files::Source,
    text::IntoText,
};
use compile::module::compile_module_file;
use explore::{explore, ExploreResult};
use fs::Fs;
use normal_file::NormalFile;
use std::{
    ffi::OsStr,
    path::Path,
    process::exit,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Barrier, Mutex,
    },
    time::Instant,
};
use thousands::Separable;
use walkdir::{DirEntry, WalkDir};

const NUM_THREADS: usize = 8;

pub fn compile_workspace(compiler: &mut Compiler, folder_path: &Path) {
    let compiler = compiler;

    let start_time = Instant::now();
    let files_processed = AtomicU64::new(0);
    let bytes_processed = AtomicU64::new(0);
    let num_files_failed = AtomicU64::new(0);

    let fs = Fs::new();

    let ExploreResult {
        module_files,
        normal_files,
    } = explore(&fs, folder_path);

    let num_threads = (normal_files.len() + module_files.len()).min(NUM_THREADS);
    let all_modules_done = Barrier::new(num_threads);
    let normal_files = Mutex::new(normal_files);
    let module_files = Mutex::new(module_files);
    let has_module_errors = AtomicBool::new(false);

    std::thread::scope(|scope| {
        for _ in 0..num_threads {
            scope.spawn(|| {
                // ===== Process module files =====
                loop {
                    // CAREFUL: Lock doesn't immediately drop unless we do it this way (while loop is not equivalent)
                    let Some(module_file) = module_files.lock().unwrap().pop() else {
                        break;
                    };

                    let Ok(did_bytes) = compile_module_file(compiler, &fs, &module_file) else {
                        num_files_failed.fetch_add(1, Ordering::Relaxed);
                        continue;
                    };

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
                    let Some(normal_file) = normal_files.lock().unwrap().pop() else {
                        break;
                    };

                    let Ok(did_bytes) = compile_normal_file(compiler, &fs, &normal_file) else {
                        num_files_failed.fetch_add(1, Ordering::Relaxed);
                        continue;
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

    if let Some(version) = compiler.version.get() {
        println!("[{}] Adept {}", folder_path.display(), version);
    } else {
        eprintln!("error: No Adept version was specified!, Use `pragma => adept(c\"3.0\")` at the top of your module file");
        exit(1);
    }

    compiler.diagnostics.push(WarningDiagnostic::plain(
        "Module system is not fully implemented yet",
    ));

    // Print summary:

    let in_how_many_seconds = start_time.elapsed().as_millis() as f64 / 1000.0;

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

fn compile_normal_file(
    compiler: &Compiler,
    _fs: &Fs,
    normal_file: &NormalFile,
) -> Result<usize, ()> {
    let path = &normal_file.path;

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

pub fn old_compile_workspace(compiler: &mut Compiler, folder_path: &Path) {
    let source_file_cache = compiler.source_file_cache;

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
                    parse_into(lexer, source_file_cache, key, ast),
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

            let mut parser = c::parser::Parser::new(
                c::parser::Input::new(lexed, source_file_cache, key),
                compiler.diagnostics,
            );

            let file_id = if let Some(ast) = &mut ast {
                exit_unless(parser.parse_into(ast), source_file_cache)
            } else {
                let (new_ast, file_id) = exit_unless(parser.parse(), source_file_cache);
                ast = Some(new_ast);
                file_id
            };

            // Translate preprocessor #define object macros
            let ast_file = ast
                .as_mut()
                .expect("ast to exist")
                .get_mut(file_id)
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
                            ast_file.helper_exprs.insert(
                                define_name.clone(),
                                ast::HelperExpr {
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

    let diagnostics = compiler.diagnostics;

    exit_unless(
        unsafe {
            llvm_backend(
                compiler,
                &ir_module,
                &resolved_ast,
                &output_object_filepath,
                &output_binary_filepath,
                diagnostics,
            )
        },
        source_file_cache,
    );
}
