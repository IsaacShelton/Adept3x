use super::c_code::c_code;
use crate::normal_file::{NormalFile, NormalFileKind};
use append_only_vec::AppendOnlyVec;
use ast::AstFile;
use build_ast::parse;
use build_c_ast::CFileType;
use build_token::Lexer;
use compiler::Compiler;
use data_units::ByteUnits;
use diagnostics::{ErrorDiagnostic, Show, into_show};
use fs_tree::FsNodeId;
use infinite_iterator::InfiniteIteratorPeeker;
use text::{CharacterInfiniteIterator, CharacterPeeker};

pub fn compile_normal_file(
    compiler: &Compiler,
    normal_file: &NormalFile,
    out_ast_files: &AppendOnlyVec<(FsNodeId, AstFile)>,
) -> Result<ByteUnits, Box<dyn Show>> {
    let path = &normal_file.path;

    let content = std::fs::read_to_string(path)
        .map_err(ErrorDiagnostic::plain)
        .map_err(into_show)?;

    let source_files = &compiler.source_files;
    let key = source_files.add(path.clone(), content);
    let content = source_files.get(key).content();
    let text = CharacterPeeker::new(CharacterInfiniteIterator::new(content.chars(), key));

    match &normal_file.kind {
        NormalFileKind::Adept => {
            let lexer = InfiniteIteratorPeeker::new(Lexer::new(text));
            out_ast_files.push((
                normal_file.fs_node_id,
                parse(lexer, source_files, key).map_err(into_show)?,
            ));
        }
        NormalFileKind::CSource => {
            out_ast_files.push((
                normal_file.fs_node_id,
                c_code(compiler, text, key, CFileType::Source)?,
            ));
        }
        NormalFileKind::CHeader => {
            out_ast_files.push((
                normal_file.fs_node_id,
                c_code(compiler, text, key, CFileType::Header)?,
            ));
        }
    }

    Ok(ByteUnits::of(content.len().try_into().unwrap()))
}
