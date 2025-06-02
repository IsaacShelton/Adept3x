pub mod c_code;
pub mod module;
pub mod normal;

use super::file::CodeFile;
use append_only_vec::AppendOnlyVec;
use ast::{ConformBehavior, RawAstFile};
use ast_workspace_settings::Settings;
use compiler::Compiler;
use data_units::ByteUnits;
use diagnostics::{ErrorDiagnostic, Show};
use fs_tree::{Fs, FsNodeId};
use infinite_iterator::InfinitePeekable;
use module::compile_rest_module_file;
use normal::compile_normal_file;
use source_files::Source;
use std::collections::HashMap;
use token::Token;

pub fn compile_code_file<'a, I: InfinitePeekable<Token>>(
    compiler: &Compiler,
    code_file: CodeFile<'a, I>,
    fs: &Fs,
    module_folders: &HashMap<FsNodeId, &Settings>,
    out_ast_files: &AppendOnlyVec<(FsNodeId, RawAstFile)>,
) -> Result<ByteUnits, Box<(dyn Show + 'static)>> {
    let fs_node_id = code_file.fs_node_id();

    let Some(settings) = get_owning_module_settings(fs_node_id, fs, module_folders) else {
        return Err(Box::new(ErrorDiagnostic::new(
            format!(
                "File '{}' is not in a module",
                code_file.path().to_string_lossy()
            ),
            Source::internal(),
        )));
    };

    let conform_behavior = ConformBehavior::Adept(settings.c_integer_assumptions());

    match code_file {
        CodeFile::Normal(normal_file) => {
            compile_normal_file(compiler, &normal_file, conform_behavior, out_ast_files)
        }
        CodeFile::Module(module_file, rest) => {
            compile_rest_module_file(&module_file, rest, conform_behavior, out_ast_files)
        }
    }
}

fn get_owning_module_settings<'a, 'b: 'a>(
    fs_node_id: FsNodeId,
    fs: &'b Fs,
    module_folders: &'b HashMap<FsNodeId, &'a Settings>,
) -> Option<&'a Settings> {
    let mut fs_node_id = fs_node_id;

    loop {
        if let Some(found) = module_folders.get(&fs_node_id) {
            return Some(found);
        }

        if let Some(parent) = fs.get(fs_node_id).parent {
            fs_node_id = parent;
        } else {
            break;
        }
    }

    None
}
