use super::header::{c_code, CFileType};
use crate::{
    ast::AstFile,
    compiler::Compiler,
    data_units::ByteUnits,
    diagnostics::{ErrorDiagnostic, WarningDiagnostic},
    inflow::IntoInflow,
    lexer::Lexer,
    line_column::Location,
    parser::parse,
    show::{into_show, Show},
    source_files::Source,
    text::{IntoText, IntoTextStream},
    workspace::{
        fs::FsNodeId,
        normal_file::{NormalFile, NormalFileKind},
    },
};
use append_only_vec::AppendOnlyVec;

pub fn compile_normal_file(
    compiler: &Compiler,
    normal_file: &NormalFile,
    out_ast_files: &AppendOnlyVec<(FsNodeId, AstFile)>,
) -> Result<ByteUnits, Box<(dyn Show + 'static)>> {
    let path = &normal_file.path;

    let content = std::fs::read_to_string(path)
        .map_err(ErrorDiagnostic::plain)
        .map_err(into_show)?;

    let source_files = &compiler.source_files;
    let key = source_files.add(path.clone(), content);
    let content = source_files.get(key).content();
    let text = content.chars().into_text_stream(key).into_text();

    match &normal_file.kind {
        NormalFileKind::Adept => {
            out_ast_files.push((
                normal_file.fs_node_id,
                parse(Lexer::new(text).into_inflow(), source_files, key).map_err(into_show)?,
            ));
        }
        NormalFileKind::CSource => {
            compiler.diagnostics.push(WarningDiagnostic::new(
                "c source files are currently treated the same as headers",
                Source::new(key, Location { line: 1, column: 1 }),
            ));

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
