use ast::RawAstFile;
use source_files::Source;

mod parse;
mod run;

pub struct PragmaSection {
    pub ast_file: RawAstFile,
    pub pragma_source: Source,
}
