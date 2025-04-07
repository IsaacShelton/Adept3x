use ast::AstFile;
use source_files::Source;

mod parse;
mod run;

pub struct PragmaSection {
    pub ast_file: AstFile,
    pub pragma_source: Source,
}
