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
use std::{ffi::OsString, fs::create_dir_all, path::Path, process::exit};

pub fn compile_single_file_only(
    compiler: &mut Compiler,
    project_folder: &Path,
    filename: &str,
    filepath: &Path,
) {
    let source_files = compiler.source_files;

    let project_name = filepath.file_stem().map(OsString::from).unwrap_or_else(|| {
        std::env::current_dir()
            .ok()
            .map(|dir| {
                dir.file_name()
                    .map(OsString::from)
                    .unwrap_or_else(|| OsString::from("main"))
            })
            .unwrap_or_else(|| OsString::from("main"))
    });

    let bin_folder = project_folder.join("bin");
    let obj_folder = project_folder.join("obj");

    create_dir_all(&bin_folder).expect("failed to create bin folder");
    create_dir_all(&obj_folder).expect("failed to create obj folder");

    let exe_filepath = bin_folder.join(compiler.target.default_executable_name(&project_name));
    let obj_filepath = obj_folder.join(compiler.target.default_object_file_name(&project_name));

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
                &obj_filepath,
                &exe_filepath,
                &compiler.diagnostics,
            )
        },
        source_files,
    );

    compiler.maybe_execute_result(&exe_filepath);
}
