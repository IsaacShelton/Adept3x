#![allow(dead_code)]

mod ast;
mod borrow;
mod c;
mod cli;
mod data_units;
mod inflow;
mod interpreter;
mod ir;
mod lexer;
mod lexical_utils;
mod line_column;
mod llvm_backend;
mod look_ahead;
mod lower;
mod parser;
mod repeating_last;
mod resolve;
mod resolved;
mod show;
mod source_file_cache;
mod tag;
mod target_info;
mod text;
mod token;
mod try_insert_index_map;

use crate::ast::Ast;
use crate::c::parser::{Input, Parser};
use crate::c::preprocessor::{DefineKind, PreToken, PreTokenKind};
use crate::c::translate_expr;
use crate::inflow::{InflowTools, IntoInflow, IntoInflowStream};
use crate::interpreter::syscall_handler::{BuildSystemSyscallHandler, ProjectKind};
use crate::ir::InterpreterSyscallKind;
use crate::source_file_cache::SourceFileCache;
use crate::tag::Tag;
use crate::text::IntoText;
use ast::{IntegerBits, IntegerSign, Source};
use c::token::CToken;
use cli::{BuildCommand, BuildOptions, NewCommand};
use indexmap::IndexMap;
use indoc::indoc;
use interpreter::Interpreter;
use lexer::Lexer;
use llvm_backend::llvm_backend;
use lower::lower;
use parser::{parse, parse_into};
use resolve::resolve;
use show::Show;
use std::fmt;
use std::io;
use std::path::Path;
use std::process::exit;
use std::{ffi::OsStr, fs::metadata};
use target_info::TargetInfo;
use walkdir::{DirEntry, WalkDir};

fn main() {
    let args = match cli::Command::parse_env_args() {
        Ok(args) => args,
        Err(()) => exit(1),
    };

    match args.kind {
        cli::CommandKind::Build(build_command) => build_project(build_command),
        cli::CommandKind::New(new_command) => new_project(new_command),
    };
}

fn build_project(build_command: BuildCommand) {
    let BuildCommand { filename, options } = build_command;
    let source_file_cache = SourceFileCache::new();
    let filepath = Path::new(&filename);

    // TODO: Determine this based on triple
    let target_info = TargetInfo {
        kind: target_info::TargetInfoKind::AARCH64,
        ms_abi: false,
        is_darwin: true,
    };

    match metadata(filepath) {
        Ok(metadata) if metadata.is_dir() => {
            compile_project(&options, target_info, &source_file_cache, filepath);
        }
        _ => {
            if !filepath.is_file() {
                eprintln!("Expected filename to be path to file");
                exit(1);
            }

            if filepath.extension().unwrap_or_default() == "h" {
                let key = source_file_cache.add_or_exit(&filename);
                let text = source_file_cache.get(key).content().chars().into_text(key);
                let preprocessed =
                    exit_unless(c::preprocessor::preprocess(text), &source_file_cache);
                println!("{:?}", preprocessed);
                return;
            }

            let project_folder = filepath.parent().unwrap();
            compile(
                &options,
                target_info,
                &source_file_cache,
                project_folder,
                &filename,
            );
        }
    }
}

struct ErrorFormatter<W: io::Write> {
    writer: W,
}

impl<W: io::Write> fmt::Write for ErrorFormatter<W> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.writer.write_all(s.as_bytes()).map_err(|_| fmt::Error)
    }
}

fn exit_unless<T, E: Show>(result: Result<T, E>, source_file_cache: &SourceFileCache) -> T {
    match result {
        Ok(value) => value,
        Err(err) => {
            let mut message = String::new();
            err.show(&mut message, source_file_cache)
                .expect("show error message");
            eprintln!("{}", message);
            exit(1);
        }
    }
}

fn compile_project(
    options: &BuildOptions,
    target_info: TargetInfo,
    source_file_cache: &SourceFileCache,
    filepath: &Path,
) {
    let folder_path = filepath;
    let walker = WalkDir::new(filepath).min_depth(1).into_iter();
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

        let filename = entry.path().to_string_lossy().to_string();
        println!("[=] {}", filename);

        let key = source_file_cache.add_or_exit(&filename);
        let content = source_file_cache.get(key).content();

        if !is_header {
            let lexer = Lexer::new(content.chars());

            if let Some(ast) = &mut ast {
                exit_unless(
                    parse_into(lexer, &source_file_cache, key, ast, filename.to_string()),
                    source_file_cache,
                );
            } else {
                ast = Some(exit_unless(
                    parse(lexer, &source_file_cache, key),
                    source_file_cache,
                ));
            }
        } else {
            let (preprocessed, defines, eof_source) = exit_unless(
                c::preprocessor::preprocess(content.chars().into_text(key)),
                source_file_cache,
            );

            let lexed = lex(preprocessed, eof_source);

            let (file_id, mut parser) = if let Some(ast) = &mut ast {
                let mut parser = Parser::new(Input::new(lexed, source_file_cache, key));
                (
                    exit_unless(parser.parse_into(ast, filename), source_file_cache),
                    parser,
                )
            } else {
                let mut parser = Parser::new(Input::new(lexed, source_file_cache, key));
                let (new_ast, file_id) = exit_unless(parser.parse(), source_file_cache);
                ast = Some(new_ast);
                (file_id, parser)
            };

            // Translate preprocessor #define object macros
            let ast_file = ast
                .as_mut()
                .expect("ast to exist")
                .files
                .get_mut(&file_id)
                .expect("recently added file to exist");

            for (define_name, define) in defines.iter() {
                match &define.kind {
                    DefineKind::ObjectMacro(expanded_replacement, _placeholder_affinity) => {
                        let lexed_replacement =
                            lex(expanded_replacement.clone(), Source::internal());
                        parser.switch_input(lexed_replacement);

                        if let Ok(value) = parser
                            .parse_expr_singular()
                            .and_then(|expr| translate_expr(ast_file, parser.typedefs(), &expr))
                        {
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

    let ast = if let Some(ast) = ast {
        ast
    } else {
        eprintln!("must have at least one adept file in directory in order to compile");
        exit(1);
    };

    let resolved_ast = exit_unless(resolve(&ast, options), source_file_cache);

    let ir_module = exit_unless(
        lower(options, &resolved_ast, target_info),
        source_file_cache,
    );

    exit_unless(
        unsafe {
            llvm_backend(
                options,
                &ir_module,
                &output_object_filepath,
                &output_binary_filepath,
            )
        },
        source_file_cache,
    );
}

fn compile(
    options: &BuildOptions,
    target_info: TargetInfo,
    source_file_cache: &SourceFileCache,
    project_folder: &Path,
    filename: &str,
) {
    let output_binary_filepath = project_folder.join("a.out");
    let output_object_filepath = project_folder.join("a.o");

    let key = match source_file_cache.add(&filename) {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Failed to open file {}", filename);
            exit(1);
        }
    };

    let content = source_file_cache.get(key).content();

    let mut ast = exit_unless(
        parse(Lexer::new(content.chars()), &source_file_cache, key),
        source_file_cache,
    );

    if options.interpret {
        setup_build_system_interpreter_symbols(&mut ast);
    }

    let resolved_ast = exit_unless(resolve(&ast, options), source_file_cache);

    let ir_module = exit_unless(
        lower(options, &resolved_ast, target_info),
        source_file_cache,
    );

    if options.interpret {
        run_build_system_interpreter(&resolved_ast, &ir_module);
    } else {
        exit_unless(
            unsafe {
                llvm_backend(
                    options,
                    &ir_module,
                    &output_object_filepath,
                    &output_binary_filepath,
                )
            },
            source_file_cache,
        );
    }
}

fn new_project(new_command: NewCommand) {
    if let Err(_) = std::fs::create_dir(&new_command.project_name) {
        eprintln!(
            "Failed to create project directory '{}'",
            &new_command.project_name
        );
        exit(1);
    }

    let imports = indoc! {r#"
        import std::prelude
    "#};

    let main = indoc! {r#"

        func main {
            println("Hello World!")
        }
    "#};

    let lock = indoc! {r#"
        std::prelude 1.0 731f4cbc9ba52451245d8f67961b640111e522972a6a4eff97c88f7ff07b0b59
    "#};

    put_file(&new_command.project_name, "_.imports", imports);
    put_file(&new_command.project_name, "_.lock", lock);
    put_file(&new_command.project_name, "main.adept", main);
    println!("Project created!");
}

fn put_file(directory_name: &str, filename: &str, content: &str) {
    let path = std::path::Path::new(directory_name).join(filename);

    if let Err(_) = std::fs::write(&path, content) {
        eprintln!("Failed to create {} file", filename);
        exit(1);
    }
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with("."))
        .unwrap_or(false)
}

fn lex(preprocessed: Vec<PreToken>, eof_source: Source) -> Vec<CToken> {
    c::Lexer::new(
        preprocessed
            .into_iter()
            .into_inflow_stream(PreToken::new(PreTokenKind::EndOfSequence, eof_source))
            .into_inflow(),
    )
    .collect_vec(true)
}

fn setup_build_system_interpreter_symbols(ast: &mut Ast) {
    // We assume only working with single file for now
    assert_eq!(ast.files.len(), 1);

    let source = Source::internal();
    let void = ast::TypeKind::Void.at(Source::internal());
    let ptr_u8 = ast::TypeKind::Pointer(Box::new(
        ast::TypeKind::Integer {
            bits: IntegerBits::Bits8,
            sign: IntegerSign::Unsigned,
        }
        .at(source),
    ))
    .at(source);

    // Call to function we actually care about
    let call = ast::ExprKind::Call(Box::new(ast::Call {
        function_name: "main".into(),
        arguments: vec![],
        expected_to_return: Some(void.clone()),
    }))
    .at(Source::internal());

    // Create entry point for interpreter which will make the call
    let (_, file) = ast.files.iter_mut().next().unwrap();
    file.functions.push(ast::Function {
        name: "<interpreter entry point>".into(),
        parameters: ast::Parameters::default(),
        return_type: void.clone(),
        stmts: vec![ast::StmtKind::Return(Some(call)).at(Source::internal())],
        is_foreign: false,
        source,
        abide_abi: false,
        tag: Some(Tag::InterpreterEntryPoint),
    });

    file.enums.insert(
        "ProjectKind".into(),
        ast::Enum {
            backing_type: Some(
                ast::TypeKind::Integer {
                    bits: IntegerBits::Bits64,
                    sign: IntegerSign::Unsigned,
                }
                .at(source),
            ),
            source,
            members: IndexMap::from_iter([
                (
                    "ConsoleApp".into(),
                    ast::EnumMember {
                        value: (ProjectKind::ConsoleApp as u64).into(),
                        explicit_value: true,
                    },
                ),
                (
                    "WindowedApp".into(),
                    ast::EnumMember {
                        value: (ProjectKind::WindowedApp as u64).into(),
                        explicit_value: true,
                    },
                ),
            ]),
        },
    );

    file.structures.push(ast::Structure {
        name: "Project".into(),
        fields: IndexMap::from_iter([(
            "kind".into(),
            ast::Field {
                ast_type: ast::TypeKind::Named("ProjectKind".into()).at(source),
                privacy: ast::Privacy::Public,
                source,
            },
        )]),
        is_packed: false,
        prefer_pod: false,
        source,
    });

    file.functions.push(ast::Function {
        name: "println".into(),
        parameters: ast::Parameters {
            required: vec![ast::Parameter::new("message".into(), ptr_u8.clone())],
            is_cstyle_vararg: false,
        },
        return_type: void.clone(),
        stmts: vec![ast::StmtKind::Expr(
            ast::ExprKind::InterpreterSyscall(Box::new(ast::InterpreterSyscall {
                kind: InterpreterSyscallKind::Println,
                args: vec![(
                    ptr_u8.clone(),
                    ast::ExprKind::Variable("message".into()).at(source),
                )],
                result_type: void.clone(),
            }))
            .at(source),
        )
        .at(source)],
        abide_abi: false,
        is_foreign: false,
        source,
        tag: None,
    });

    file.functions.push(ast::Function {
        name: "project".into(),
        parameters: ast::Parameters {
            required: vec![
                ast::Parameter::new("name".into(), ptr_u8.clone()),
                ast::Parameter::new(
                    "project".into(),
                    ast::TypeKind::Named("Project".into()).at(source),
                ),
            ],
            is_cstyle_vararg: false,
        },
        return_type: void.clone(),
        stmts: vec![ast::StmtKind::Expr(
            ast::ExprKind::InterpreterSyscall(Box::new(ast::InterpreterSyscall {
                kind: InterpreterSyscallKind::BuildAddProject,
                args: vec![
                    (
                        ptr_u8.clone(),
                        ast::ExprKind::Variable("name".into()).at(source),
                    ),
                    (
                        ast::TypeKind::Named("ProjectKind".into()).at(source),
                        ast::ExprKind::Member(
                            Box::new(ast::ExprKind::Variable("project".into()).at(source)),
                            "kind".into(),
                        )
                        .at(source),
                    ),
                ],
                result_type: void.clone(),
            }))
            .at(source),
        )
        .at(source)],
        abide_abi: false,
        is_foreign: false,
        source,
        tag: None,
    })
}

fn run_build_system_interpreter(resolved_ast: &resolved::Ast<'_>, ir_module: &ir::Module) {
    let (interpreter_entry_point, _fn) = resolved_ast
        .functions
        .iter()
        .find(|(_, f)| f.tag == Some(Tag::InterpreterEntryPoint))
        .unwrap();

    let max_steps = Some(1_000_000);
    let handler = BuildSystemSyscallHandler::default();
    let mut interpreter = Interpreter::new(handler, &ir_module, max_steps);

    match interpreter.run(interpreter_entry_point) {
        Ok(result) => assert!(result.is_literal() && result.unwrap_literal().is_void()),
        Err(err) => {
            eprintln!("build script error: {}", err);
            return;
        }
    }

    println!("Building:\n{:#?}", interpreter.syscall_handler.projects);
}
