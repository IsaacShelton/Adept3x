pub mod c_code;
pub mod module;
pub mod normal;

use super::file::CodeFile;
use append_only_vec::AppendOnlyVec;
use ast::AstFile;
use compiler::Compiler;
use data_units::ByteUnits;
use diagnostics::Show;
use fs_tree::FsNodeId;
use infinite_iterator::InfinitePeekable;
use module::compile_rest_module_file;
use normal::compile_normal_file;
use token::Token;

pub fn compile_code_file<'a, I: InfinitePeekable<Token>>(
    compiler: &Compiler,
    code_file: CodeFile<'a, I>,
    out_ast_files: &AppendOnlyVec<(FsNodeId, AstFile)>,
) -> Result<ByteUnits, Box<(dyn Show + 'static)>> {
    match code_file {
        CodeFile::Normal(normal_file) => compile_normal_file(compiler, &normal_file, out_ast_files),
        CodeFile::Module(module_file, rest) => {
            compile_rest_module_file(&module_file, rest, out_ast_files)
        }
    }
}
