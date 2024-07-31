/*
    ========================  single_file_only/mod.rs  ========================
    Module for compiling projects that are only a single file
    ---------------------------------------------------------------------------
*/

use crate::{
    compiler::Compiler,
    exit_unless,
    inflow::IntoInflow,
    interpreter_env::{run_build_system_interpreter, setup_build_system_interpreter_symbols},
    lexer::Lexer,
    llvm_backend::llvm_backend,
    lower::lower,
    parser::parse,
    resolve::resolve,
    text::{IntoTextStream, TextStream},
};
use std::{path::Path, process::exit};

pub fn compile_single_file_only(compiler: &Compiler, project_folder: &Path, filename: &str) {
    let source_file_cache = &compiler.source_file_cache;
    let output_binary_filepath = project_folder.join("a.out");
    let output_object_filepath = project_folder.join("a.o");

    let content = std::fs::read_to_string(filename)
        .map_err(|err| {
            eprintln!("{}", err);
            exit(1);
        })
        .unwrap();

    let key = source_file_cache.add(filename.into(), content);
    let content = source_file_cache.get(key).content();
    let text = content.chars().into_text_stream(key).into_text();

    let mut ast = exit_unless(
        parse(Lexer::new(text).into_inflow(), source_file_cache, key),
        source_file_cache,
    );

    if compiler.options.interpret {
        setup_build_system_interpreter_symbols(&mut ast);
    }

    let resolved_ast = exit_unless(resolve(&ast, &compiler.options), source_file_cache);

    let ir_module = exit_unless(
        lower(&compiler.options, &resolved_ast, &compiler.target_info),
        source_file_cache,
    );

    if compiler.options.interpret {
        run_build_system_interpreter(&resolved_ast, &ir_module);
        return;
    }

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
