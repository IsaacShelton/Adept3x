/*
    ========================  single_file_only/mod.rs  ========================
    Module for compiling projects that are only a single file
    ---------------------------------------------------------------------------
*/

use crate::{
    ast::AstWorkspace,
    compiler::Compiler,
    exit_unless,
    inflow::IntoInflow,
    interpreter_env::{run_build_system_interpreter, setup_build_system_interpreter_symbols},
    lexer::Lexer,
    llvm_backend::llvm_backend,
    lower::lower,
    parser::parse,
    resolve::resolve,
    text::{IntoText, IntoTextStream},
    workspace::fs::Fs,
};
use indexmap::IndexMap;
use std::{path::Path, process::exit};

pub fn compile_single_file_only(
    compiler: &mut Compiler,
    project_folder: &Path,
    filename: &str,
    filepath: &Path,
) {
    let source_files = compiler.source_files;
    let output_binary_filepath = project_folder.join("a.out");
    let output_object_filepath = project_folder.join("a.o");

    let content = std::fs::read_to_string(filename)
        .map_err(|err| {
            eprintln!("{}", err);
            exit(1);
        })
        .unwrap();

    let key = source_files.add(filename.into(), content);
    let content = source_files.get(key).content();
    let text = content.chars().into_text_stream(key).into_text();

    let fs = Fs::new();
    let fs_node_id = fs.insert(filepath, None).expect("inserted");

    let mut ast_file = exit_unless(
        parse(Lexer::new(text).into_inflow(), source_files, key),
        source_files,
    );

    if compiler.options.interpret {
        setup_build_system_interpreter_symbols(&mut ast_file);
    }

    let files = IndexMap::from_iter(std::iter::once((fs_node_id, ast_file)));
    let mut workspace = AstWorkspace::new(fs, files, compiler.source_files, None);

    let resolved_ast = exit_unless(resolve(&mut workspace, &compiler.options), source_files);

    let ir_module = exit_unless(
        lower(&compiler.options, &resolved_ast, &compiler.target),
        source_files,
    );

    if compiler.options.interpret {
        match run_build_system_interpreter(&resolved_ast, &ir_module) {
            Ok(_) => return,
            Err(err) => {
                eprintln!("{}", err);
                exit(1);
            }
        }
    }

    exit_unless(
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
        source_files,
    );

    compiler.maybe_execute_result(&output_binary_filepath);
}
