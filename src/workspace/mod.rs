/*
    ===========================  workspace/mod.rs  ============================
    Module for compiling entire workspaces
    ---------------------------------------------------------------------------
*/

mod explore;
mod fs;
mod normal_file;

use crate::{
    ast::{self, Ast, AstFile, FileId, Function, Parameters, Source, StmtKind, TypeKind},
    c::{
        self,
        lexer::lex_c_code,
        preprocessor::{DefineKind, Preprocessed},
        translate_expr,
    },
    cli::BuildOptions,
    compiler::Compiler,
    diagnostics::WarningDiagnostic,
    exit_unless,
    inflow::{Inflow, IntoInflow},
    interpreter_env::{run_build_system_interpreter, setup_build_system_interpreter_symbols},
    lexer::Lexer,
    llvm_backend::llvm_backend,
    lower::lower,
    parser::{self, error::ParseErrorKind, parse, parse_into, Input},
    resolve::resolve,
    show::{into_show, Show},
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
        atomic::{AtomicBool, AtomicU64, Ordering},
        Barrier, Mutex, OnceLock,
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
    let num_files_failed = AtomicU64::new(0);

    let ExploreResult {
        root_node,
        module_files,
        normal_files,
    } = explore(folder_path);

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

                    let Ok(did_bytes) = compile_module_file(compiler, &root_node, &module_file)
                    else {
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

                    let Ok(did_bytes) = compile_normal_file(compiler, &root_node, &normal_file)
                    else {
                        num_files_failed.fetch_add(1, Ordering::Relaxed);
                        continue;
                    };

                    files_processed.fetch_add(1, Ordering::Relaxed);
                    bytes_processed.fetch_add(did_bytes.try_into().unwrap(), Ordering::Relaxed);
                }
            });
        }
    });

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

struct PragmaSection<'a, I: Inflow<Token>> {
    pub ast: Ast<'a>,
    pub rest_input: Option<Input<'a, I>>,
    pub pragma_source: Source,
}

impl<'a, I: Inflow<Token>> PragmaSection<'a, I> {
    pub fn run(mut self, base_compiler: &Compiler) -> Result<Option<Input<'a, I>>, Box<dyn Show>> {
        let compiler = Compiler {
            options: BuildOptions {
                emit_llvm_ir: false,
                emit_ir: false,
                interpret: true,
                coerce_main_signature: false,
            },
            target_info: base_compiler.target_info.clone(),
            source_file_cache: base_compiler.source_file_cache,
            diagnostics: base_compiler.diagnostics,
            version: OnceLock::new(),
        };

        setup_build_system_interpreter_symbols(&mut self.ast);

        let resolved_ast = resolve(&self.ast, &compiler.options).map_err(into_show)?;

        let ir_module =
            lower(&compiler.options, &resolved_ast, &compiler.target_info).map_err(into_show)?;

        if let Ok(interpreter) = run_build_system_interpreter(&resolved_ast, &ir_module) {
            if let Some(version) = interpreter.syscall_handler.version {
                if base_compiler.version.try_insert(version).is_err() {
                    return Err(into_show(
                        ParseErrorKind::Other {
                            message: "Adept version was already specified".into(),
                        }
                        .at(self.pragma_source),
                    ));
                }
            }
        }

        Ok(self.rest_input)
    }
}

fn parse_pragma_section<'a, I: Inflow<Token> + 'a>(
    mut input: Input<'a, I>,
) -> Result<PragmaSection<'a, I>, Box<dyn Show>> {
    input.ignore_newlines();

    let Some(pragma_source) = input.eat_remember(TokenKind::PragmaKeyword) else {
        return Err(Box::new(
            ParseErrorKind::Other {
                message: "Expected 'pragma' at beginning of module file".into(),
            }
            .at(input.peek().source),
        ));
    };

    input.ignore_newlines();

    let mut ast_file = AstFile::new();
    let mut parser = parser::Parser::new(input);

    if parser.input.eat(TokenKind::OpenCurly) {
        // "Whole-file" mode

        while !parser.input.peek_is(TokenKind::CloseCurly) {
            parser
                .parse_top_level(&mut ast_file, vec![])
                .map_err(into_show)?;

            parser.input.ignore_newlines();
        }

        if !parser.input.eat(TokenKind::CloseCurly) {
            return Err(Box::new(
                ParseErrorKind::Expected {
                    expected: "'}'".into(),
                    for_reason: Some("to close pragma section".into()),
                    got: parser.input.peek().to_string(),
                }
                .at(parser.input.peek().source),
            ));
        }
    } else if let Some(source) = parser.input.eat_remember(TokenKind::FatArrow) {
        let is_block = parser.input.peek_is(TokenKind::OpenCurly);

        let stmts = if is_block {
            // "Inside-main-only" mode
            parser.parse_block("pragma").map_err(into_show)?
        } else {
            // "Single-expression" mode
            let expr = parser.parse_expr().map_err(into_show)?;
            let expr_source = expr.source;
            vec![StmtKind::Expr(expr).at(expr_source)]
        };

        ast_file.functions.push(Function {
            name: "main".into(),
            parameters: Parameters {
                required: vec![],
                is_cstyle_vararg: false,
            },
            return_type: TypeKind::Void.at(source),
            stmts,
            is_foreign: false,
            source,
            abide_abi: false,
            tag: None,
        });
    } else {
        return Err(Box::new(
            ParseErrorKind::Expected {
                expected: "'=>' or '{' after 'pragma' keyword".into(),
                for_reason: None,
                got: parser.input.peek().to_string(),
            }
            .at(parser.input.peek().source),
        ));
    }

    // Leave input unfinished
    input = parser.input;

    // Create AST from ast file
    let filename = input.filename();
    let file_id = FileId::Local(filename.into());
    let mut ast = Ast::new(filename.into(), input.source_file_cache());
    ast.files.insert(file_id, ast_file);

    Ok(PragmaSection {
        ast,
        rest_input: Some(input),
        pragma_source,
    })
}

fn compile_module_file(compiler: &Compiler, _root_node: &FsNode, path: &Path) -> Result<usize, ()> {
    let content = std::fs::read_to_string(path).map_err(|err| {
        eprintln!("{}", err);
        ()
    })?;

    let source_file_cache = &compiler.source_file_cache;
    let key = source_file_cache.add(path.to_path_buf(), content);
    let content = source_file_cache.get(key).content();

    let text = content.chars().into_text_stream(key).into_text();
    let lexer = Lexer::new(text).into_inflow();
    let mut input = Input::new(lexer, compiler.source_file_cache, key);
    input.ignore_newlines();

    while input.peek_is(TokenKind::PragmaKeyword) {
        input = match parse_pragma_section(input).and_then(|section| section.run(compiler)) {
            Ok(Some(rest)) => rest,
            Ok(None) => break,
            Err(err) => {
                let mut s = String::new();
                err.show(&mut s, source_file_cache).unwrap();
                eprintln!("{}", s);
                return Err(());
            }
        };

        input.ignore_newlines();
    }

    Ok(content.len())
}

fn compile_normal_file(
    compiler: &Compiler,
    _root_node: &FsNode,
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

            let mut parser = c::parser::Parser::new(
                c::parser::Input::new(lexed, source_file_cache, key),
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
