/*
    ===========================  workspace/mod.rs  ============================
    Module for compiling entire workspaces
    ---------------------------------------------------------------------------
*/

use crate::{
    ast::{self, Source},
    c::{
        self,
        parser::{Input, Parser},
        preprocessor::{DefineKind, PreToken, PreTokenKind, Preprocessed},
        token::CToken,
        translate_expr,
    },
    compiler::Compiler,
    exit_unless,
    inflow::{InflowTools, IntoInflow, IntoInflowStream},
    lexer::Lexer,
    llvm_backend::llvm_backend,
    lower::lower,
    parser::{parse, parse_into},
    resolve::resolve,
    text::IntoText,
};
use std::{ffi::OsStr, path::Path, process::exit};
use walkdir::{DirEntry, WalkDir};

pub fn compile_workspace(compiler: &Compiler, folder_path: &Path) {
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

        let filename = entry.path().to_string_lossy().to_string();
        println!("[=] {filename}");

        let key = source_file_cache.add_or_exit(&filename);
        let content = source_file_cache.get(key).content();

        if !is_header {
            let lexer = Lexer::new(content.chars());

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

fn lex_c_code(preprocessed: Vec<PreToken>, eof_source: Source) -> Vec<CToken> {
    c::Lexer::new(
        preprocessed
            .into_iter()
            .into_inflow_stream(PreToken::new(PreTokenKind::EndOfSequence, eof_source))
            .into_inflow(),
    )
    .collect_vec(true)
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with("."))
        .unwrap_or(false)
}
