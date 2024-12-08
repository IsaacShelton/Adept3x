pub mod header;
pub mod module;
pub mod normal;

use super::{file::CodeFile, fs::FsNodeId};
use crate::{
    ast::AstFile, compiler::Compiler, data_units::ByteUnits, inflow::Inflow, show::Show,
    token::Token,
};
use append_only_vec::AppendOnlyVec;
use module::compile_rest_module_file;
use normal::compile_normal_file;

pub fn compile_code_file<'a, I: Inflow<Token>>(
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
